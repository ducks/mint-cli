# Local account verification

Run from your own interactive terminal. Never paste credentials, tokens,
cookies, HARs, or account response bodies into chat or issue reports.

1. Build with `nix build` or `cargo build --locked`.
2. Unlock your Linux Secret Service backend.
3. Run `nix run . -- login` or `target/debug/mint-cli login`.
   Enter the phone number and password only at the private local prompts.
   If requested, enter the authenticator code at the next prompt.
4. Run `mint-cli status`, `mint-cli usage`, and `mint-cli plans list`.
   API schemas can differ. Missing fields cause a generic schema error;
   raw response data is deliberately not printed or logged.
5. Try `mint-cli refresh` only after the initial session works. It rotates
   credentials and updates the keyring. It does not retry account requests.
6. Run `mint-cli logout` when finished. If remote revocation cannot be
   confirmed, the command reports that separately from local deletion.

Report only command name, success/failure, sanitized HTTP status, and which
schema fields are missing. Do not post a session snapshot. Do not enable
HTTP wire logging or share a browser profile. If direct login requires an
anti-abuse/browser/text challenge, stop. The next discovery step is an
explicitly authorized local authenticated browser session with the user
completing the provider's normal challenge. There is no challenge bypass.

Renewal needs further authenticated read-only observation of the quote,
saved payment method, and billing history. Do not click Pay or submit any
purchase request during discovery. Complete price and transaction outcome
semantics must be documented before adding a financial command.

## Reported blocker

The user's local CLI login returned HTTP 403. Browser login also fails,
with many 403 responses and misleading password-error text. Prior changes
to VPN/browser configuration produced intermittent success, not a reliable
fix. Do not keep retrying credentials or assume the password is wrong.
No successful authenticated session is available for renewal verification.

## Current testing boundary

Covered with synthetic HTTP mocks: authentication success, MFA challenge
and resubmission, invalid credentials, token expiry/rotation, cookies,
session invalidation, malformed responses, rate limiting, account/usage
parsing, plan/price parsing, no retries, redirects, response-size bounds,
and redaction. CLI tests cover output escaping and command boundaries.

Pending: a real Secret Service integration test, live auth/MFA and cookie
lifetimes, linked accounts, and actual renewal quote/receipt behavior.
Transaction confirmation and ambiguous purchase reconciliation tests are
pending the verified payment implementation. Tests never make purchases.
