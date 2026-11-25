use crate::{Client, Error};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Method;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Session {
    pub(crate) account_id: String,
    pub(crate) token: SecretString,
    expires_at: Option<u64>,
}
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Session([REDACTED])")
    }
}
#[derive(Debug)]
pub enum LoginOutcome {
    Authenticated(Session),
    MfaRequired,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub version: u8,
    pub account_id: String,
    pub token: String,
    pub cookies: String,
}

fn claims(token: &str) -> Option<Value> {
    let segment = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(segment).ok()?;
    serde_json::from_slice(&bytes).ok()
}
fn id(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_u64().map(|v| v.to_string()))
}
impl Session {
    pub(crate) fn from_parts(account_id: String, token: SecretString) -> Result<Self, Error> {
        if account_id.is_empty()
            || account_id.len() > 128
            || !account_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
            || token.expose_secret().is_empty()
            || !token
                .expose_secret()
                .bytes()
                .all(|b| (33..=126).contains(&b))
        {
            return Err(Error::InvalidSession);
        }
        let expires_at = claims(token.expose_secret()).and_then(|v| v["exp"].as_u64());
        Ok(Self {
            account_id,
            token,
            expires_at,
        })
    }
    pub fn ensure_valid(&self) -> Result<(), Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Expired)?
            .as_secs();
        if self
            .expires_at
            .is_some_and(|exp| exp <= now.saturating_add(30))
        {
            Err(Error::Expired)
        } else {
            Ok(())
        }
    }
}
impl Client {
    /// One attempt only. No fake anti-abuse identifier and no legacy fallback.
    pub async fn login(
        &self,
        phone: &SecretString,
        password: &SecretString,
        mfa: Option<&SecretString>,
    ) -> Result<LoginOutcome, Error> {
        let phone = phone.expose_secret();
        if phone.len() != 10 || !phone.bytes().all(|c| c.is_ascii_digit()) {
            return Err(Error::InvalidSession);
        }
        if mfa.is_some_and(|v| {
            v.expose_secret().len() != 6 || !v.expose_secret().bytes().all(|c| c.is_ascii_digit())
        }) {
            return Err(Error::InvalidSession);
        }
        let mut body =
            json!({"msisdn":phone,"password":password.expose_secret(),"subscriberType":"PHONE"});
        if let Some(mfa) = mfa {
            body["mfaToken"] = json!(mfa.expose_secret());
        }
        let data: Value = self
            .request(Method::POST, "v2/mint/login", None, Some(&body))
            .await?;
        if data["mfa"].as_bool() == Some(true) {
            return Ok(LoginOutcome::MfaRequired);
        }
        let token = data["token"].as_str().ok_or(Error::InvalidResponse)?;
        let account_id = id(&data["userId"])
            .or_else(|| id(&data["id"]))
            .ok_or(Error::InvalidResponse)?;
        if data["subscriberType"]
            .as_str()
            .is_some_and(|s| s != "PHONE")
        {
            return Err(Error::InvalidResponse);
        }
        Ok(LoginOutcome::Authenticated(Session::from_parts(
            account_id,
            token.to_owned().into(),
        )?))
    }

    /// Explicit refresh only: callers must save rotated cookies even on failure.
    pub async fn refresh(&self, session: &mut Session) -> Result<(), Error> {
        let data: Value = self
            .request(Method::POST, "v2/mint/refresh", None, None)
            .await?;
        let token = data["token"].as_str().ok_or(Error::InvalidResponse)?;
        let refreshed_id = claims(token)
            .and_then(|c| id(&c["phone"]))
            .ok_or(Error::InvalidResponse)?;
        if refreshed_id != session.account_id {
            return Err(Error::InvalidSession);
        }
        *session = Session::from_parts(refreshed_id, token.to_owned().into())?;
        Ok(())
    }

    pub async fn logout(&self, session: &Session) -> Result<(), Error> {
        // Revoke even an expired token, using its session cookies.
        self.request::<Value>(
            Method::GET,
            "v2/mint/logout?subscriberType=PHONE",
            Some(&session.token),
            None,
        )
        .await
        .map(|_| ())
    }
}
