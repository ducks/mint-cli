# Discovery log

## 2026-11-25

1. Reviewed the three requested references before implementation. Pinned
   both public GitHub revisions in api.md. Bot README reports breakage;
   Parecki's article establishes only historical token/usage behavior.
2. Reviewed service terms, site terms and acceptable use policy. Flagged
   the automated-access restriction. User clarified directed personal use
   and asked to continue. No claim of Mint authorization is made.
3. Opened the unauthenticated login page in an isolated browser context.
   Observed the current gateway and public outage call. Browser asset
   downloads succeeded; a direct curl asset request returned 403. No
   headers/cookies were replayed to defeat that response. Continued reading
   assets served normally in the browser. Browser telemetry occurred during
   page loading; none is implemented in the client.
4. Saved public script bodies temporarily outside the repository. Extracted
   endpoint facts only. Browser initially restarted before body retrieval;
   a new isolated context was opened. No account login took place.
5. Verified GET /v1/mint/outages?subscriberType=PHONE directly with curl,
   no Authorization or Cookie header. HTTP 200, JSON:
   `{"outage":false,"message":"Service is up","planned":false}`.
   This is service availability, not account or authentication verification.
6. Inspected APP, LOGIN, MFA, and roaming code. Static evidence identifies
   V2 cookie refresh and authenticator MFA login resubmission. Account
   schemas are partial and live account calls remain unverified.
7. Followed public checkout chunk references from the build manifest.
   Chunk 384 had no runtime mapping and returned 404; the other explicitly
   referenced chunks loaded. Inspected both checkout variants and found
   GET quotes and POST recharge/plan operations. Braintree device data,
   split payments, taxes and auto-recharge require further verification.
8. Created docs/api.md before the Rust workspace. User explicitly requested
   login and account commands now despite pending live verification.
9. Implemented read-only account calls, explicit login/MFA, refresh and
   logout. Financial mutations remain absent. Mocks use synthetic data.

No authenticated endpoint, renewal quote, purchase, or live MFA challenge
has been verified. Do not upgrade S/H/M evidence to V on the basis of tests.

10. Added a read-only `plans quote` operation from the checkout code. It
    preserves itemCost, creditCard tax/recoveryFee/total, wallet and renewal
    balance fields without asserting verified monetary units or final
    charge semantics. There is still no purchase operation.

11. The built Rust CLI successfully fetched the public outage response.
    User performed local login and reported Access Denied (HTTP 403). This
    verifies a rejection, not successful authentication or invalid password.
    User reports the browser displays incorrect phone/password despite
    known credentials. Browser HTTP status is not yet available. LOGIN's
    error mapper defaults unrecognized errors to that same message. Added
    bounded, allowlisted 403 classification without raw body logging.

12. User reports many browser HTTP 403s and intermittent success followed
    by failures when changing VPN/browser setup. This does not isolate a
    VPN, browser, account, gateway, or anti-abuse cause. No further login
    attempts were made by the agent. Successful account auth is blocked.
