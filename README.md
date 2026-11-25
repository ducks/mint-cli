# mint-cli

A lightweight, privacy-conscious, unofficial Rust client for Mint Mobile.
The goal is account inspection and subscription renewal from a terminal
when the web interface fails in privacy-focused browsers.

**Early development:** the public service-status operation was verified
live. Login and account operations are implemented from Mint's public
frontend and tested with mocks, but have not been verified with a real
account. A local login attempt returned HTTP 403; browser login also
failed for the user. The cause remains unresolved. Browser challenges may
prevent direct login. Subscription
purchases are not enabled yet; see the [renewal blocker](docs/api.md#renewal-gate).

## Install

Stable Rust (1.89 or newer), Linux, and libsecret's `secret-tool` are needed:

```sh
cargo install --locked --path crates/mint-cli
```

Enable and unlock a Secret Service backend, such as GNOME Keyring or
KeePassXC with Secret Service integration. NixOS users need a configured
session D-Bus and Secret Service; the CLI does not start a daemon.
Conventional Linux installations need no Nix at runtime.

```sh
nix develop
nix build
nix run . -- --help
nix flake check
```

Enable Nix `nix-command` and `flakes` experimental features if needed.
The flake pins build and development tools and includes libsecret. Its
package wrapper locates `secret-tool`; it does not provide a keyring daemon.

## Commands

```sh
mint-cli service-status --json
mint-cli login
mint-cli status --json
mint-cli account
mint-cli usage --json
mint-cli plans list --json
mint-cli plans current
mint-cli plans quote --json
mint-cli refresh
mint-cli logout
mint-cli logout --local
```

Login prompts privately for a ten-digit phone number and password, then
an authenticator code if Mint requests MFA. No credential arguments,
environment variables, or piped credential input are accepted. One initial
attempt and at most one MFA submission are made. A 403, 412, provider/VPN
restriction, or browser challenge stops the flow; no alternate endpoint or
mobile-app impersonation is attempted.

`refresh` explicitly exchanges session cookies for a fresh access token.
It does not resend your password. Expired or invalidated sessions never
cause automatic login or request retries. If refresh is rejected, log in
again. `logout --local` deletes only the local session; it does not revoke
Mint's server-side session. Normal logout attempts remote revocation and
removes local credentials even when revocation fails, reporting the failure.

JSON is one object on stdout, with `schema_version`, `kind`, and `data`.
Errors use stderr and a nonzero exit status. Schema version 1 keeps numeric
source values, explicit `null` for unknown optional fields, string IDs,
and snake_case keys. Unknown server fields are not forwarded. In Nushell:

```nu
mint-cli usage --json | from json | get data.used_mb
```

Human output uses plain text, no ANSI colors, progress animations, or
terminal control sequences, including under `NO_COLOR` and redirection.
Account output contains personal information: protect any redirected files.
`plans quote` makes read-only requests for the current subscription, plans,
and recharge quote. It reports API-provided price components in native
units with unverified semantics, and leaves the effective date and future
auto-renew implications unknown. It cannot authorize or submit a payment.
Plan costs are source values, not an authoritative final renewal price.
Usage units follow the historical API's MB labels; precise byte conversion
is unverified.

## Privacy and credentials

- No telemetry, analytics, intermediary server, polling, or background task.
- Direct TLS-verified requests to Mint; no insecure TLS option or redirects.
- Passwords and MFA codes are never persisted. Secret wrappers redact debug
  output and erase their owned buffers on drop; HTTP/JSON libraries can
  create temporary copies, so this is not a guarantee against memory dumps.
- Access tokens and session cookies are saved using Linux Secret Service.
  There is no plaintext credential fallback. Keyring encryption and access
  control depend on your backend and desktop session.
- One local profile. Commands serialize session changes using a lock in
  `$XDG_STATE_HOME/mint-cli` (default `~/.local/state/mint-cli`). The lock
  contains no secrets. No configuration or cache files are required.
- Errors omit response bodies, tokens, account URLs, and headers. The CLI
  does not initialize tracing or HTTP debug logging.
- Mint still sees your IP address and requests. A VPN is supported as normal
  network routing, but provider restrictions are respected.

## Compatibility and renewal

Only primary phone accounts are currently supported. No automatic family,
Minternet, or tablet discovery. Account operations are **static/mocked**,
not live verified. See [API inventory](docs/api.md),
[discovery log](docs/discovery-log.md), and [architecture](docs/architecture.md).

Renewal is a primary objective. Quote and purchase routes were found, but
live quote semantics, saved-method requirements, device-data requirements,
receipts, and reconciliation remain unverified. No purchase, plan change,
wallet top-up, or roaming activation command is exposed. The next step is
a user-directed authenticated read-only verification, followed by a quote
review. A purchase will require explicit confirmation, and ambiguous
transactions must never be retried automatically.

Mint's site terms restrict automated access. The applicability to a
user-directed client remains unresolved; this project has no provider
endorsement or guarantee of compatibility. It does not bypass authentication,
MFA, browser challenges, or other access controls.

## Develop

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Tests use local HTTP mocks and synthetic fixtures, never real accounts or
financial transactions. Never commit tokens, HARs, browser sessions, or
account responses. See [local verification](docs/local-verification.md).

Original code is MIT licensed. Reference implementations were investigated
for protocol facts; no implementation code or frontend bundles were copied
into this repository. Mint Mobile is a trademark of its owner. This project
is not affiliated with or endorsed by Mint Mobile.
