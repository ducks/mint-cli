use crate::{Client, Error, auth::Session};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    #[serde(rename(deserialize = "remainingHighSpeedData"))]
    pub remaining_mb: f64,
    #[serde(rename(deserialize = "usageHighSpeedData"))]
    pub used_mb: f64,
}
impl Client {
    pub async fn usage(&self, session: &Session) -> Result<Usage, Error> {
        let usage: Usage = self
            .account_request(
                session,
                Method::POST,
                "v2",
                "/usage",
                Some(&json!({"types":["data"],"subscriberType":"PHONE"})),
            )
            .await?;
        if !usage.used_mb.is_finite()
            || !usage.remaining_mb.is_finite()
            || usage.used_mb < 0.0
            || usage.remaining_mb < 0.0
        {
            return Err(Error::InvalidResponse);
        }
        Ok(usage)
    }
}
