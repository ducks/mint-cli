use crate::{Error, auth::Session};
use reqwest::{
    Method,
    header::{AUTHORIZATION, HeaderValue},
};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use secrecy::{ExposeSecret, SecretString};
use serde::{Serialize, de::DeserializeOwned};
use std::{sync::Arc, time::Duration};

const BASE: &str = "https://mint-gateway.mintmobile.com/";
const MAX_BODY: usize = 1024 * 1024;

pub struct Client {
    http: reqwest::Client,
    pub(crate) cookies: Arc<CookieStoreMutex>,
    base: String,
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct ServiceStatus {
    pub outage: bool,
    pub message: String,
    pub planned: bool,
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        Self::build(BASE, CookieStore::default())
    }

    fn build(base: &str, cookies: CookieStore) -> Result<Self, Error> {
        let cookies = Arc::new(CookieStoreMutex::new(cookies));
        let http = reqwest::Client::builder()
            .user_agent("mint-cli/0.1.0 (unofficial)")
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .cookie_provider(cookies.clone())
            .build()
            .map_err(|_| Error::Transport)?;
        Ok(Self {
            http,
            cookies,
            base: base.to_owned(),
        })
    }

    pub fn restore(saved: &SecretString) -> Result<(Self, Session), Error> {
        let snapshot: crate::auth::Snapshot =
            serde_json::from_str(saved.expose_secret()).map_err(|_| Error::InvalidSession)?;
        if snapshot.version != 1 {
            return Err(Error::InvalidSession);
        }
        let jar = cookie_store::serde::json::load(snapshot.cookies.as_bytes())
            .map_err(|_| Error::InvalidSession)?;
        let session = Session::from_parts(snapshot.account_id, snapshot.token.into())?;
        Ok((Self::build(BASE, jar)?, session))
    }

    pub fn snapshot(&self, session: &Session) -> Result<SecretString, Error> {
        let jar = self.cookies.lock().map_err(|_| Error::InvalidSession)?;
        let mut bytes = Vec::new();
        cookie_store::serde::json::save_incl_expired_and_nonpersistent(&jar, &mut bytes)
            .map_err(|_| Error::InvalidSession)?;
        let cookies = String::from_utf8(bytes).map_err(|_| Error::InvalidSession)?;
        let snapshot = crate::auth::Snapshot {
            version: 1,
            account_id: session.account_id.clone(),
            token: session.token.expose_secret().to_owned(),
            cookies,
        };
        serde_json::to_string(&snapshot)
            .map(Into::into)
            .map_err(|_| Error::InvalidSession)
    }

    pub async fn service_status(&self) -> Result<ServiceStatus, Error> {
        self.request(
            Method::GET,
            "v1/mint/outages?subscriberType=PHONE",
            None,
            None,
        )
        .await
    }

    pub(crate) async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        token: Option<&SecretString>,
        body: Option<&serde_json::Value>,
    ) -> Result<T, Error> {
        let trace_id = format!("web-mint/{}", uuid::Uuid::new_v4());
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.base))
            .header("Accept", "application/json")
            .header("Channel", "web-am")
            .header("kaena-channel", "jq49mud7wtn9zmr")
            .header("kaena-sid", &trace_id)
            .header("kaena-pid", &trace_id)
            .header("kaena-tid", &trace_id);
        if let Some(token) = token {
            let mut value = HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
                .map_err(|_| Error::InvalidSession)?;
            value.set_sensitive(true);
            request = request.header(AUTHORIZATION, value);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = request.send().await.map_err(|_| Error::Transport)?;
        let status = response.status().as_u16();
        match status {
            200..=299 => {}
            401 => return Err(Error::Unauthorized),
            403 => return Err(classify_forbidden(response).await),
            412 => return Err(Error::ChallengeRequired),
            429 => {
                return Err(Error::RateLimited {
                    retry_after_seconds: response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse().ok()),
                });
            }
            _ => return Err(Error::Http(status)),
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BODY as u64)
        {
            return Err(Error::ResponseTooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Transport)? {
            if bytes.len() + chunk.len() > MAX_BODY {
                return Err(Error::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)
    }

    pub(crate) async fn account_request<T: DeserializeOwned>(
        &self,
        session: &Session,
        method: Method,
        version: &str,
        suffix: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<T, Error> {
        session.ensure_valid()?;
        self.request(
            method,
            &format!("{version}/mint/account/{}{suffix}", session.account_id),
            Some(&session.token),
            body,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) fn test_client(base: &str) -> Self {
        Self::build(&format!("{base}/"), CookieStore::default()).unwrap()
    }
}

/// Classify only allowlisted signals, never expose arbitrary body/header values.
async fn classify_forbidden(mut response: reqwest::Response) -> Error {
    if response
        .headers()
        .get("cf-mitigated")
        .is_some_and(|v| v == "challenge")
    {
        return Error::BrowserChallenge;
    }
    if response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/html"))
    {
        return Error::HtmlAccessDenied;
    }
    let mut bytes = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        if bytes.len() + chunk.len() > 8192 {
            return Error::AccessDenied;
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes == b"BAD_ISP" {
        return Error::ProviderRestricted;
    }
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        if value.as_str() == Some("BAD_ISP") {
            return Error::ProviderRestricted;
        }
        if value.get("reason").and_then(|v| v.as_str()) == Some("portInPending") {
            return Error::PortInPending;
        }
    }
    Error::AccessDenied
}
