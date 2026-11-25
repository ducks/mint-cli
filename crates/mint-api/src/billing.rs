//! Renewal/payment endpoints are documented but not implemented until a live
//! quote and reconciliation semantics are verified. No financial POST exists.
use serde::{Deserialize, Serialize};

/// A partial view of the frontend's cost structure, never a purchase authority.
#[derive(Debug, Deserialize, Serialize)]
pub struct CostBreakdown {
    pub total: serde_json::Number,
    pub tax: serde_json::Number,
    #[serde(rename(deserialize = "recoveryFee"))]
    pub recovery_fee: serde_json::Number,
}

use crate::{Client, Error, auth::Session};
use reqwest::Method;

/// Source quote values only. Currency, completeness and effective date still
/// require live verification. This type cannot be submitted as a purchase.
#[derive(Debug, Deserialize, Serialize)]
pub struct RenewalQuote {
    #[serde(rename(deserialize = "itemCost"))]
    pub item_cost: serde_json::Number,
    #[serde(rename(deserialize = "itemCostSubtotal"))]
    pub item_cost_subtotal: Option<serde_json::Number>,
    #[serde(rename(deserialize = "renewalBalance"))]
    pub renewal_balance: Option<serde_json::Number>,
    #[serde(rename(deserialize = "creditCard"))]
    pub credit_card: CostBreakdown,
    pub wallet: Option<CostBreakdown>,
}
impl Client {
    /// Read-only GET. Never changes next plan or auto-renew, and never pays.
    pub async fn renewal_quote(&self, session: &Session) -> Result<RenewalQuote, Error> {
        let quote: RenewalQuote = self
            .account_request(
                session,
                Method::GET,
                "v1",
                "/checkout/recharge?subscriberType=PHONE",
                None,
            )
            .await?;
        let mut numbers = vec![
            &quote.item_cost,
            &quote.credit_card.total,
            &quote.credit_card.tax,
            &quote.credit_card.recovery_fee,
        ];
        numbers.extend(quote.item_cost_subtotal.iter());
        numbers.extend(quote.renewal_balance.iter());
        if let Some(wallet) = &quote.wallet {
            numbers.extend([&wallet.total, &wallet.tax, &wallet.recovery_fee]);
        }
        if numbers
            .iter()
            .any(|n| n.as_f64().is_none_or(|v| !v.is_finite() || v < 0.0))
        {
            return Err(Error::InvalidResponse);
        }
        Ok(quote)
    }
}
