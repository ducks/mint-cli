//! Unofficial Mint Mobile API. Account operations are statically discovered,
//! not yet verified against an authenticated live account. See docs/api.md.
pub mod account;
pub mod auth;
pub mod billing;
pub mod client;
pub mod error;
pub mod plans;
pub mod roaming;
pub mod usage;
pub use client::Client;
pub use error::Error;

#[cfg(test)]
mod tests;
