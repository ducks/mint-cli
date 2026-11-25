use thiserror::Error;

/// Errors deliberately exclude URLs, response bodies, headers and secrets.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("network request failed; no automatic retry was attempted")]
    Transport,
    #[error("HTTP 401: authentication rejected or session expired; no retry attempted")]
    Unauthorized,
    #[error(
        "HTTP 403: access denied; this does not establish whether the password was checked; no retry attempted"
    )]
    AccessDenied,
    #[error(
        "HTTP 403: an HTML access-denied page was returned; password validity is unknown; no retry attempted"
    )]
    HtmlAccessDenied,
    #[error(
        "HTTP 403: the provider explicitly requires a browser challenge; complete its normal flow"
    )]
    BrowserChallenge,
    #[error(
        "HTTP 403: provider reported BAD_ISP; no attempt to circumvent the restriction was made"
    )]
    ProviderRestricted,
    #[error("HTTP 403: provider reported a pending number transfer")]
    PortInPending,
    #[error("HTTP 412: Mint requires a browser or text verification challenge")]
    ChallengeRequired,
    #[error("rate limited; retry-after seconds: {retry_after_seconds:?}; no retry attempted")]
    RateLimited { retry_after_seconds: Option<u64> },
    #[error("unexpected HTTP status {0}; no retry attempted")]
    Http(u16),
    #[error("API response does not match the discovered schema")]
    InvalidResponse,
    #[error("response exceeded the size limit")]
    ResponseTooLarge,
    #[error("invalid session or request identifier")]
    InvalidSession,
    #[error("session expired; log in again")]
    Expired,
}
