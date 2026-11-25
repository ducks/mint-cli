use crate::{Client, Error, auth::Session};
use reqwest::Method;
use serde::{Deserialize, Serialize};

/// Deliberate allowlist: never forwards arbitrary server fields or credentials.
#[derive(Debug, Deserialize, Serialize)]
pub struct Account {
    pub status: String,
    pub msisdn: Option<String>,
    #[serde(rename(deserialize = "firstName"))]
    pub first_name: Option<String>,
    #[serde(rename(deserialize = "autoRecharge"))]
    pub auto_renew: Option<bool>,
    pub plan: CurrentPlan,
    pub wallet: Option<serde_json::Number>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentPlan {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: String,
    pub months: Option<u32>,
    pub exp: Option<u64>,
    #[serde(rename(deserialize = "endOfCycle"))]
    pub end_of_cycle: Option<u64>,
}
impl Client {
    pub async fn account(&self, session: &Session) -> Result<Account, Error> {
        self.account_request(session, Method::GET, "v1", "?subscriberType=PHONE", None)
            .await
    }
}

pub(crate) fn deserialize_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = serde_json::Value::deserialize(d)?;
    match value {
        serde_json::Value::String(s) if !s.is_empty() => Ok(s),
        serde_json::Value::Number(n) if n.is_u64() => Ok(n.to_string()),
        _ => Err(serde::de::Error::custom("expected an identifier")),
    }
}
