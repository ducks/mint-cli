# API inventory

Discovery date: 2026-11-25. This is an unofficial, incomplete protocol
inventory, not an API contract. No account credentials were used during
initial discovery. No financial requests were sent.

## Evidence and verification labels

- **H**: historical third-party implementation; not live verified here.
- **S**: statically discovered in Mint's public frontend; not live verified.
- **M**: tested against synthetic HTTP fixtures only, not live verification.
- **V**: observed successful live request, with scope explicitly stated.

Sources:

- [HA source][ha], revision `4408940602c50822a3dd85d494ef060bf899026c`.
- [Telegram source][bot], revision
  `2251214ac2614324b4f0537873e0a3c138a33ab9`. Its README says it is broken.
- [Aaron Parecki, 2020][aaron]: token login and usage in MB; no endpoint
  paths or schemas supplied. This is historical context only.
- **APP**: public Next.js `_app-c438e5e110f2000c.js`, build
  `TmDzO91CG7UVQmYO_qo32`, retrieved through an unauthenticated browser.
  SHA-256: `368d6fee79f909b6a25abc32a04463457cca0a782127bbf58974099dd72b3370`.
- **LOGIN**: `pages/login-c183ac182641b7d0.js` and
  `pages/login/authentication-code-7e6be8dd3b456247.js`.
- **CHECKOUT**: `96-5c64cdf04c1e13a6.js`,
  `8685-5cf8910fa729bf5f.js`, `3088.9cd5e3c16ecafe10.js`.
- **ROAM**: `863-e20a922ded21b633.js`.

Asset paths above are relative to
`https://my.mintmobile.com/_next/static/chunks/`. Source bundles are not
redistributed. Static findings describe the code shipped on this date;
feature flags and server configuration can select different behavior.

## Hosts, headers, and lifecycle

Current frontend base: `https://mint-gateway.mintmobile.com/`. Paths mix
`v1/mint/`, `v2/mint/`, `v3/mint/`, and unversioned `mint/`. Do not infer
that the same operation exists at another version.

Historical bot base: `https://api.mintsim.com/v1/mint/`. The user-provided
`https://w3b-api.ultramobile.com/v2/mint/login` is an unverified discovery
lead only; no request was sent to it.

The browser sends `Channel: web-am`, `kaena-channel: jq49mud7wtn9zmr`, and
`kaena-sid`, `kaena-pid`, `kaena-tid`, each set to the same newly generated
`web-mint/<UUID>` per request. Authenticated requests send a bearer token.
These are observed headers; necessity of each is unverified. The browser
includes cookies. POST/PUT/PATCH/DELETE bodies and object-form GET queries
have `subscriberType` injected (normally `PHONE`). String-form queries in
the frontend do not receive that injection.

Web login uses `x-ipq-requestid`, obtained by the frontend's anti-abuse
integration. A direct client must not manufacture it, spoof a mobile app,
reuse the references' static app bearer, or evade a challenge. Initial
login without an available identifier is unverified; stop if challenged.
VPN/provider rejection may appear as `BAD_ISP`; do not route around it.

V2 refresh sends cookies and no JSON refresh token. Cookie names, lifetime,
rotation and server-side revocation behavior remain unverified. JWT `exp`
is a scheduling hint only, not a signature check or authorization proof.
The frontend reads `phone`/`internet` claims for refreshed account identity.
Do not assume the HA reference's 900-second fallback is an API guarantee.

The HA V1 flow uses `{msisdn,password,subscriberType:"PHONE"}` (or username),
a static application bearer, mobile channel and an Android user agent.
It consumes `token` or `accessToken`, `refreshToken`, JWT `sub`/`userId` and
`exp`. These headers and fallback behaviors are not used by this project.

## Endpoint inventory

`A` below means account bearer and browser cookies as applicable. All S/H
response shapes are partial observations, not complete schemas. `unknown`
means no response schema was established, not an empty JSON object.
No authenticated success is V yet. A user-directed local V2 login attempt
returned HTTP 403; this does not establish password validity. Library tests add M evidence only.

| Method / path | Auth | Request | Response evidence | Side effects | Source / status |
| --- | --- | --- | --- | --- | --- |
| GET `/v1/mint/outages` | Public | query `subscriberType=PHONE` | `{outage:bool,message:string,planned:bool}` | None observed | APP; V browser and direct HTTP 200 |
| POST `/v2/mint/login` | Password; anti-abuse context | `{msisdn:string,password:string,subscriberType:"PHONE",mfaToken?:string}`; optional `x-ipq-requestid` | `{id?:number/string,userId?:number/string,token?:string,mfa?:bool,subscriberType?:string}`; cookies unknown | Creates session; failed attempts may lock account | APP/LOGIN; S |
| POST `/v2/mint/refresh` | Session cookies | no body in refresh helper; `x-ipq-requestid` | `{token:string}`; JWT identity claims | Rotates session credentials, details unknown | APP; S |
| GET `/v2/mint/logout` | A | subscriberType query | unknown | Invalidates session; scope unknown | APP; S |
| GET `/v1/mint/account/{id}` | A | optional `fields[]` | `status,msisdn,firstName,plan,autoRecharge,wallet`; linked products possible | No mutation known | APP + HA; S/H |
| POST `/v2/mint/account/{id}/usage` | A | `{types:["data"],subscriberType:"PHONE"}` | `remainingHighSpeedData,usageHighSpeedData` numbers; linked product objects | Read operation despite POST | APP + HA; S/H |
| GET `/mint/usage` | A | `bucket[]`, subscriberType | unknown | No mutation known | APP; S |
| GET `/v1/mint/account/{id}/plans` | A | none | `{id,nextPlan,availablePlans:[...]}`; may include internet | No mutation known | APP + HA; S/H |
| GET `/v1/mint/account/{id}/plans/{planId}` | A | none | `{plan:{id,data,...}}` | No mutation known | APP; S |
| GET `/v1/mint/account/{id}/plan/summary` | A | none | unknown | No mutation known | APP; S |
| GET `/v1/mint/plans` | Public in frontend | `planType`, `detailed=true` | unknown full shape | No mutation known | APP; S |
| GET `/v1/mint/plans/{planId}` | Public in frontend | none | `{plan:{boltOns:[...],...}}` | No mutation known | APP; S |
| GET `/mint/plan/pricing` | A | `planId[]` | unknown | No mutation known | APP; S |
| POST `/v1/mint/account/{id}/plans` | A | `{id,subscriberType?}` | unknown | Schedules next plan; not a quote | APP; S |
| GET `/v1/mint/account/{id}/checkout/{item}` | A | `ID?`, `payment?`, `zip?`, `immediate?`, subscriberType; item includes `recharge`, `plan` | `itemCost,itemCostSubtotal,renewalBalance,creditCard:{total,tax,recoveryFee},wallet`; full schema unverified | Quote; no mutation known | CHECKOUT; S |
| POST `/v1/mint/account/{id}/checkout/recharge` | A | `{subscriberType,payment,zip,deviceData}`; optional `payments:["wallet","cc"]` | unknown receipt and reconciliation identifiers | **Financial charge and renewal** | CHECKOUT; S, not implemented |
| POST `/v1/mint/account/{id}/checkout/plan` | A | as recharge plus `ID` | unknown | **Financial charge and plan change** | CHECKOUT; S, not implemented |
| POST `/v1/mint/account/{id}/auto-recharge` | A | `{autoRecharge:bool,subscriberType}` | unknown | Changes future automatic billing | APP; S, not implemented |
| PATCH `/mint/account/subscriptions/plan` | A | `{status}` | unknown | Changes subscription state | APP; S, not implemented |
| GET `/v1/mint/account/{id}/billing` | A | subscriberType | Payment method object; frontend handles 404 as missing | No mutation known | APP; S |
| GET `/v1/mint/account/{id}/payment/summary` | A | none | unknown | No mutation known | APP; S |
| GET `/v1/mint/account/{id}/billing/history` | A | `start` default 1, `limit` default 10, `end` JS milliseconds; subscriberType | `{history:[...],internet?:{history:[...]}}`; entry schema unknown | No mutation known | APP; S |
| GET `/v1/mint/products/international-roaming-pass` | A in frontend | subscriberType | Array consumed as products; entries include days | No mutation known | ROAM; S |
| GET `/v1/mint/account/{id}/international-roaming-pass` | A | subscriberType | unknown | No mutation known | ROAM; S |
| POST `/v1/mint/account/{id}/international-roaming-pass/{code}/activate` | A | subscriberType | unknown | Starts purchased pass validity | ROAM; S, not implemented |
| POST `/v1/mint/account/{id}/roaming/balance/transfer` | A | subscriberType | unknown | Transfers balance | APP; S, not implemented |
| GET `/v1/mint/account/{id}/multi-line` | A | none | `{activeMembers:[{id,msisdn,currentPlan,nextPlan,...}]}` | No mutation known | APP + HA; S/H |
| GET `/v1/mint/account/{id}/multi-line/{member}/usage` | A | none | `{data:{remaining4G,usage4G}}` | No mutation known | APP + HA; S/H |
| GET `/v1/mint/account/{id}/multi-line/{member}/checkout/recharge` | A | none | unknown | Quote | APP; S |
| POST `/v1/mint/account/{id}/multi-line/{member}/checkout/recharge` | A | `{deviceData,subscriberType}` | unknown | **Financial renewal** | APP; S, not implemented |
| POST `/v1/mint/login` on gateway | Static app bearer in reference | msisdn/username, password, subscriberType | token/accessToken, refreshToken | Creates session | HA; H |
| POST `/v1/mint/refresh` on gateway | Static app bearer in reference | `{id,refreshToken}` | `{token,refreshToken}` | Token rotation | HA; H |
| POST `/v1/mint/login` on api.mintsim.com | Static app bearer | `{msisdn,password}` | token, refreshToken consumed | Creates session | Bot; H, marked broken |
| POST `/v1/mint/refresh` on api.mintsim.com | Static app bearer | `{msisdn,refreshToken}` | token consumed | Refresh | Bot; H, marked broken |
| GET `/v1/mint/account/{id}/data` on api.mintsim.com | Account bearer | none | `{remaining4G,usage4G}` | No mutation known | Bot; H, marked broken |

MFA enrollment endpoints in APP (`POST`, `PUT`, `DELETE`
`/v1/mint/account/{id}/settings/mfa`, and `DELETE /v3/mint/mfa`) are settings
mutations, not login challenge endpoints. Their body schemas are unresolved
and they are intentionally not implemented. Interactive login repeats V2
login with the password and a six-digit authenticator `mfaToken`.

## Errors, limits, and units

LOGIN handles 401 with optional `{warning,attempts}`, 429 for lockout, 403
with optional `reason:"portInPending"`, and 412 with `message:"PENDING"` or
`"BLOCKED"` for a text challenge. These are static observations, not live
error verification. Rate windows, quotas, Retry-After availability, and
MFA attempt budgets are unknown. Do not probe limits. One login attempt
and at most one user-supplied MFA submission are made per invocation.

Usage source fields are reported as MB by historical sources. The bot
divides by 1000 and HA by 1024. Preserve source values and label that
uncertainty; do not invent an exact byte conversion. Account timestamps
are treated as epoch seconds by HA. Plan entries observed in APP include
`id`, `count`, `contractRenewalPeriodCount`, `cost`, `data.capLTE`,
`data.capTetherLTE`, `displayName`, `immediate`, `deferred`, and subtotals.
Missing values must stay unknown, not become zero.

## Renewal gate

The individual recharge and plan purchase routes have been discovered.
The frontend obtains Braintree `deviceData`, selects `payment:"cc"` or
wallet, may combine payments, and can separately enable auto-recharge.
No device-data requirement may be bypassed or filled with invented data.

Before enabling a purchase, verify a user's quote, currency and units,
base price, tax, recovery fees, credits, final charge, selected/current
plan, duration, allowance, effective date, saved payment method, and
whether auto-renew changes. Verify receipt/reconciliation identifiers and
idempotency semantics. Static discovery alone does not establish these.
A read-only quote is not an authoritative purchase offer until those
semantics are verified. No payment POST exists in the client yet.

Future transactions must persist an in-flight record before sending,
require explicit confirmation of the displayed quote, and never retry
ambiguous outcomes. Reconcile account plus billing history before another
attempt, including after a crash or restart. No raw card collection.

## Terms review

Reviewed [site terms][terms] (October 24, 2023), [service terms][service]
(June 9, 2026), and [acceptable use policy][aup]. Site terms restrict
automated access; applicability to a user-directed personal client needs
clarification, and this project claims no provider authorization. The
user requested continued directed research. No access control, MFA,
anti-abuse challenge, or provider restriction was bypassed.

[ha]: https://github.com/ryanmac8/HA-Mint-Mobile/blob/4408940602c50822a3dd85d494ef060bf899026c/custom_components/mintmobile/api.py
[bot]: https://github.com/eralpsahin/mint-telegram-bot/blob/2251214ac2614324b4f0537873e0a3c138a33ab9/lib/mint.js
[aaron]: https://aaronparecki.com/2020/06/05/36/mint
[terms]: https://www.mintmobile.com/site-terms-of-use/
[service]: https://www.mintmobile.com/plan-terms-and-conditions/
[aup]: https://www.mintmobile.com/acceptable-use-policy/
