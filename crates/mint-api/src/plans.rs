use crate::{Client, Error, auth::Session};
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Plans {
    #[serde(deserialize_with = "crate::account::deserialize_id")]
    pub id: String,
    #[serde(
        rename(deserialize = "nextPlan"),
        default,
        deserialize_with = "optional_id"
    )]
    pub next_plan: Option<String>,
    #[serde(rename(deserialize = "availablePlans"))]
    pub available_plans: Vec<Plan>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Plan {
    #[serde(deserialize_with = "crate::account::deserialize_id")]
    pub id: String,
    #[serde(rename(deserialize = "displayName"))]
    pub display_name: Option<String>,
    pub count: Option<u32>,
    #[serde(rename(deserialize = "contractRenewalPeriodCount"))]
    pub contract_renewal_period_count: Option<u32>,
    /// Source value; not an authoritative renewal quote or inferred currency.
    pub cost: Option<serde_json::Number>,
    pub data: Option<Allowance>,
    pub immediate: Option<bool>,
    pub deferred: Option<bool>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Allowance {
    #[serde(rename(deserialize = "capLTE"))]
    pub cap_lte: Option<serde_json::Number>,
    #[serde(rename(deserialize = "capTetherLTE"))]
    pub cap_tether_lte: Option<serde_json::Number>,
}
impl Client {
    pub async fn plans(&self, session: &Session) -> Result<Plans, Error> {
        self.account_request(session, Method::GET, "v1", "/plans", None)
            .await
    }
}

fn optional_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(d)?;
    value
        .map(|v| match v {
            serde_json::Value::String(s) => Ok(s),
            serde_json::Value::Number(n) if n.is_u64() => Ok(n.to_string()),
            _ => Err(serde::de::Error::custom("expected an identifier")),
        })
        .transpose()
}
