mod commands;
mod credentials;
mod output;

use clap::{Parser, Subcommand};
use mint_api::{Client, auth::LoginOutcome};
use secrecy::SecretString;
use std::{io::IsTerminal, process::ExitCode};

#[derive(Parser)]
#[command(
    version,
    about = "Unofficial Mint Mobile client (account API compatibility unverified)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Emit one versioned JSON object. Errors go to stderr.
    #[arg(long, global = true)]
    json: bool,
}
#[derive(Subcommand)]
enum Command {
    /// Log in using private terminal prompts; supports authenticator MFA.
    Login,
    /// Revoke remote session and remove local credentials.
    Logout {
        /// Remove local credentials without contacting Mint.
        #[arg(long)]
        local: bool,
    },
    /// Explicitly refresh session cookies and access token.
    Refresh,
    /// Inspect account status and current subscription.
    Status,
    /// Inspect account details (may contain personal information).
    Account,
    /// Inspect data usage in source-reported MB.
    Usage,
    /// Inspect subscription plans; amounts are not renewal quotes.
    Plans {
        #[command(subcommand)]
        command: PlanCommand,
    },
    /// Check Mint's public outage notice (not your account status).
    ServiceStatus,
}
#[derive(Subcommand)]
enum PlanCommand {
    List,
    Current,
    /// Fetch a read-only renewal quote; units/semantics await live verification.
    Quote,
}

fn prompt(label: &str) -> Result<SecretString, String> {
    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return Err("login requires an interactive terminal; credentials cannot be supplied as arguments or piped input".into());
    }
    rpassword::prompt_password(label)
        .map(Into::into)
        .map_err(|_| "could not read private terminal input".into())
}

async fn run(cli: Cli) -> Result<(), String> {
    let _lock = if matches!(cli.command, Command::ServiceStatus) {
        None
    } else {
        Some(credentials::session_lock()?)
    };
    let store = credentials::Store;
    match cli.command {
        Command::ServiceStatus => {
            let data = Client::new()
                .map_err(|e| e.to_string())?
                .service_status()
                .await
                .map_err(|e| e.to_string())?;
            output::emit("service_status", &data, cli.json)
        }
        Command::Login => {
            if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
                return Err("login requires an interactive terminal".into());
            }
            // Check backend availability before requesting any credentials.
            store.check()?;
            let phone = prompt("Phone number (10 digits, hidden): ")?;
            let password = prompt("Password: ")?;
            let client = Client::new().map_err(|e| e.to_string())?;
            let mut result = client
                .login(&phone, &password, None)
                .await
                .map_err(|e| e.to_string())?;
            if matches!(result, LoginOutcome::MfaRequired) {
                let code = prompt("Authenticator code: ")?;
                result = client
                    .login(&phone, &password, Some(&code))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            drop(password);
            drop(phone);
            let LoginOutcome::Authenticated(session) = result else {
                return Err("MFA was not accepted; no further attempt made".into());
            };
            store.save(&client.snapshot(&session).map_err(|e| e.to_string())?)?;
            output::emit(
                "login",
                &serde_json::json!({"authenticated":true}),
                cli.json,
            )
        }
        Command::Logout { local } => {
            if local {
                store.delete()?;
            } else {
                let (client, session) =
                    Client::restore(&store.load()?).map_err(|e| e.to_string())?;
                let revoked = client.logout(&session).await;
                store.delete()?;
                revoked.map_err(|e| {
                    format!("local credentials removed; remote revocation unconfirmed: {e}")
                })?;
            }
            output::emit(
                "logout",
                &serde_json::json!({"local_credentials_removed":true,"remote_revocation_requested":!local}),
                cli.json,
            )
        }
        command => {
            let (client, mut session) =
                Client::restore(&store.load()?).map_err(|e| e.to_string())?;
            if matches!(command, Command::Refresh) {
                let result = client.refresh(&mut session).await;
                // Save even failed-response cookie rotation; never silently discard it.
                store.save(&client.snapshot(&session).map_err(|e| e.to_string())?)?;
                result.map_err(|e| e.to_string())?;
                return output::emit("refresh", &serde_json::json!({"refreshed":true}), cli.json);
            }
            let result = commands::read(&client, &session, command).await;
            // Persist server cookie updates before returning data or errors.
            store.save(&client.snapshot(&session).map_err(|e| e.to_string())?)?;
            let (kind, data) = result?;
            output::emit(kind, &data, cli.json)
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({"schema_version":1,"error":{"message":message}})
                );
            } else {
                eprintln!("mint-cli: {message}");
            }
            ExitCode::FAILURE
        }
    }
}
