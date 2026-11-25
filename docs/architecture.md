# Architecture and decisions

The workspace has two crates. `mint-api` owns transport, protocol parsing,
secret session values, and an in-memory cookie jar. It has no terminal,
credential-storage, environment-path, or CLI dependencies. `mint-cli` owns
command parsing, interactive input, Secret Service, XDG state locking, and
versioned output.

The production gateway is fixed. There is no runtime base-URL flag that
could redirect credentials. Tests use an internal constructor compiled only
under `cfg(test)` and bind wiremock on loopback. TLS validation is enabled,
redirects and transport retries disabled, and responses are capped at 1 MiB.
HTTP errors discard bodies and URLs. Unknown schemas fail without leaking
raw data. Read commands issue only the request needed by that command.

Sessions contain account identity, bearer token, expiry hint, and cookies.
`Session` Debug output is fully redacted. Passwords and MFA codes are held
in secret wrappers only for interactive login. A V2 MFA challenge causes
one explicit second login with a privately entered authenticator code.
Other challenges stop; no legacy or mobile fallback exists.

The CLI persists a versioned session snapshot as one Secret Service item.
Cookies, including nonpersistent cookies, survive CLI process exit so the
user can explicitly refresh. This deliberately extends browser-session
storage to a keyring session; server expirations still apply. No refresh
cookie name or lifetime is assumed. Cookie domain/path/secure/expiry rules
are delegated to cookie_store. Expired access-token hints prevent account
requests. Refresh is explicit and checks that the refreshed phone identity
matches the stored account. Cookie rotation is saved even on API failure.
A process lock prevents concurrent CLI commands from overwriting rotation.
Library users must arrange their own session persistence and serialization.

Login replaces the single saved profile only after a successful response.
An existing remote session is not automatically revoked during replacement.
If a backend fails after successful authentication, the CLI reports failure
and keeps no plaintext fallback; the remote session may still exist.

Machine output uses an allowlist of typed fields with a versioned envelope.
Prices and allowances preserve source values without invented units, taxes,
or totals. Human strings are escaped to prevent terminal control injection.
No color or TTY detection is needed for rendering; login requires a TTY.

## Decision log

- 2026-11-25: inspect references before code; mark historical, static,
  mocked, and live evidence independently. New HA code is not our own live
  verification and the bot explicitly reports breakage.
- 2026-11-25: use current web V2 login, not the historical app bearer or
  Android impersonation. Do not manufacture anti-abuse identifiers.
- 2026-11-25: user requested exposing implemented account operations before
  live verification. Mark compatibility clearly; no success placeholders.
- 2026-11-25: use libsecret's established Linux Secret Service frontend
  rather than adding a large D-Bus dependency tree. Tokens go over stdin;
  the Nix package wraps PATH to locate the tool. Conventional installs need
  the libsecret utility and an existing unlocked backend.
- 2026-11-25: no automatic retries, even for reads. Explicit refresh avoids
  an implicit network operation and rotated-token persistence surprises.
- 2026-11-25: defer payment mutation implementation until authenticated
  quote/receipt evidence permits safe confirmation and reconciliation.
  Do not write tests for an invented purchase protocol.
- 2026-11-25: no tracing subscriber: there is nothing useful to log yet that
  justifies an additional surface for accidental credential disclosure.

## Required before purchases

A purchase requires a complete, fresh API quote with current and selected
subscription, duration, allowance, base price, taxes/fees, final charge,
effective date, existing payment method, and auto-renewal consequences.
Keep currency arithmetic in minor integer units only after verifying units.
A dry run may fetch a quote, but never create an order or imply success.

Confirmation must bind to that exact quote. Before sending, persist a
nonsecret in-flight transaction record atomically and serialize attempts.
Any timeout, malformed receipt, unexpected status, or crash after sending
is ambiguous. Do not retry or clear the record based only on an error.
Reconcile account and billing history using verified identifiers before
allowing another purchase. These requirements are not implemented yet,
because no financial operation is exposed.
