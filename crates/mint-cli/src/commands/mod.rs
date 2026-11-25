use crate::{Command, PlanCommand};
use mint_api::{Client, auth::Session};
use serde_json::{Value, json};

pub async fn read(
    client: &Client,
    session: &Session,
    command: Command,
) -> Result<(&'static str, Value), String> {
    let result = match command {
        Command::Status => {
            let account = client.account(session).await.map_err(|e| e.to_string())?;
            (
                "status",
                json!({"status":account.status,"auto_renew":account.auto_renew,"plan":account.plan}),
            )
        }
        Command::Account => (
            "account",
            json!(client.account(session).await.map_err(|e| e.to_string())?),
        ),
        Command::Usage => (
            "usage",
            json!(client.usage(session).await.map_err(|e| e.to_string())?),
        ),
        Command::Plans {
            command: PlanCommand::List,
        } => (
            "plans",
            json!(client.plans(session).await.map_err(|e| e.to_string())?),
        ),
        Command::Plans {
            command: PlanCommand::Current,
        } => (
            "current_plan",
            json!(
                client
                    .account(session)
                    .await
                    .map_err(|e| e.to_string())?
                    .plan
            ),
        ),
        Command::Plans {
            command: PlanCommand::Quote,
        } => {
            let account = client.account(session).await.map_err(|e| e.to_string())?;
            let plans = client.plans(session).await.map_err(|e| e.to_string())?;
            let quote = client
                .renewal_quote(session)
                .await
                .map_err(|e| e.to_string())?;
            (
                "renewal_quote",
                json!({
                    "current_subscription":account.plan,
                    "scheduled_plan_id":plans.next_plan,
                    "available_plans":plans.available_plans,
                    "auto_renew_current":account.auto_renew,
                    "auto_renew_after_purchase":null,
                    "effective_date":null,
                    "amount_unit":"api_native_units_unverified",
                    "purchase_enabled":false,
                    "quote":quote,
                }),
            )
        }
        _ => return Err("invalid read command".into()),
    };
    Ok(result)
}
