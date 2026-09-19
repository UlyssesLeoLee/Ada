# Auth, Billing & RBAC Architecture — v0.4.0

> Companion to `docs/architecture/` and `docs/commercial/COMPLIANCE_SELFCHECK.md`.
> Decision-freezing document: every choice made for v0.4.0 is recorded here.

## 1. Layer model

```
+------------------------------------------------------------------+
|                  gm-console  Web SPA  (React + tokens.css)        |
|                  gm-console Mobile (Flutter)                     |
+------------------------------------------------------------------+
                              |   OIDC / SAML (browser SSO)
                              |   TOTP / WebAuthn / Passkey (MFA)
                              v
+------------------------------------------------------------------+
|              crates/ada-identity (new in v0.4.0)                  |
|              - OIDC RP     (openidconnect crate)                  |
|              - SAML 2.0 SP (samael crate)                         |
|              - WebAuthn RP (webauthn-rs crate)                    |
|              - TOTP        (totp-rs crate)                        |
|              - Passkey     (WebAuthn-resident-key flow)           |
+------------------------------------------------------------------+
                              |   JWT (RS256) + session cookie
                              v
+------------------------------------------------------------------+
|              crates/ada-m13-api-gateway (existing)                |
|              - validates JWT against ada-identity's JWKS         |
|              - propagates UserId + TenantId + Roles + Claims      |
|              - applies tenant RLS to downstream calls            |
+------------------------------------------------------------------+
        |                                                |
        v                                                v
+----------------+                            +--------------------------+
|  ada-m11-rbac  |  static role:permission     |  crates/ada-rbac-casbin |
|  (existing)    |  + LockManager + AuditSink  |  (new in v0.4.0)        |
|                |  ----- the *ground truth* --|  - casbin enforcer      |
+----------------+                            |  - policy hot-reload    |
                                               |  - ABAC attribute       |
                                               |    evaluator (tenant,   |
                                               |    time, owner, scope)  |
                                               +--------------------------+
                                                             |
                                                             v
                                                   +-------------------+
                                                   |  crates/ada-billing|
                                                   |  (new in v0.4.0)  |
                                                   |  Stripe Billing   |
                                                   |  webhooks + portal|
                                                   +-------------------+
```

## 2. Identity provider matrix

| Provider                 | Protocol              | Role          | Notes                                  |
|--------------------------|-----------------------|---------------|----------------------------------------|
| Self-host OIDC           | OpenID Connect 1.0    | IdP + RP      | Dex / Keycloak / Authentik reference   |
| Self-host LDAP           | OIDC front (via Dex)  | IdP-backed    | LDAP bind via Dex's LDAP connector     |
| Okta / Azure AD          | OIDC (preferred) + SAML | IdP (managed) | Both protocols supported               |
| Google Workspace         | OIDC                  | IdP (managed) | Standard                                |
| Customer-passkey         | WebAuthn-resident     | RP only       | "Sign in with passkey" button on /login |

Each IdP is configurable per tenant. A tenant is *either* managed by an external IdP
(saml/oidc) *or* by gm-console's own RP (passkey + passwordless). Self-host tenants start
with passkey as default and add IdP later.

## 3. MFA enrollment

| Path                              | User experience                                                 |
|-----------------------------------|------------------------------------------------------------------|
| First login                       | Passkey enrollment prompt (resident key on device)              |
| If device lacks platform passkey  | TOTP QR code → Authenticator app; WebAuthn-bound YubiKey option |
| Recovery                          | Recovery codes (10 single-use) generated once on enrollment      |
| Step-up                          | Sensitive actions (rotate billing, add IdP) require re-auth     |

The MFA surface is exposed as `Identity::require_mfa(session, action)` returning an
opaque challenge token; the api-gateway enforces it before policy evaluation.

## 4. RBAC + ABAC

m11 owns the canonical `Role` × `Permission` matrix. The new `ada-rbac-casbin` crate
mounts a Casbin enforcer that *imports* m11's matrix as its static policy base and
*adds* attribute-based rules.

```
subject := user_id
object := resource_id (e.g. pipeline:abc123)
action := m11::Action
attrs := {
    tenant_id,
    role,            // m11::Role
    is_owner,        // bool
    is_public,       // bool
    now_utc,         // time-bound policy
    request_ip,       // IP-bound policy (optional)
}
casbin.enforce(subject, object, action, attrs) -> Allow|Deny
```

Casbin policies are stored in `crates/ada-rbac-casbin/policies/` and hot-reloaded on
SIGHUP / file change. New policies are added via the api-gateway admin endpoint
`POST /admin/policies` (which itself requires `Role::Owner`).

## 5. Billing (Stripe)

`ada-billing` provides:

- **Customer mapping**: `UserId ↔ Stripe customer.id`
- **Subscription state**: `active | past_due | canceled | trialing | incomplete`
- **Webhook handler**: `POST /webhooks/stripe` (signature verified, idempotency-keyed)
- **Customer portal link**: `GET /api/v1/billing/portal`
- **Entitlement surface**: `Entitlement::for(user)` reads subscription + plan tier,
  exposes `can_use(Feature)` to api-gateway middleware.

Plans:

| Plan           | Stripe Price ID (placeholder) | Features                                |
|----------------|--------------------------------|------------------------------------------|
| `free`         | (default, no Stripe)           | 1 tenant, 5 pipelines, 14-day audit log  |
| `team`         | `price_team_monthly`           | 10 tenants, unlimited pipelines, 90-day  |
| `enterprise`   | `price_enterprise_monthly`     | SSO required, dedicated, custom SLAs    |

`ada-billing` is **not** in the request path for unauthenticated requests — it sits
behind `api-gateway` and only handles `BillingActor` (auth + tenant) calls.

## 6. Threat model (top 5)

1. **Credential stuffing against /login**: rate-limited via tower middleware; passkey
   path preferred over password; lockout after N failures per IP+UA.
2. **CSRF on /webhooks/stripe**: signature verified with `Stripe-Signature`; idempotency
   keys; webhook URL not exposed to user.
3. **Policy hot-reload TOCTOU**: reloaded enforcer is `Arc<RwLock<Enforcer>>`; mutations
   from admin endpoint require `Role::Owner` + step-up MFA.
4. **Cross-tenant data leak via JWT claim tampering**: api-gateway validates JWT against
   ada-identity's JWKS; tenant_id is taken from validated claims, never from request
   body. ada-identity's signing key is rotated via standard OIDC `kid` rollover.
5. **Stripe webhook replay**: idempotency table keyed by `event.id` + `tenant_id` —
   duplicates are silently dropped.

## 7. Data flow examples

### Login via SAML

```
browser → gm-console /api/v1/auth/saml/login?tenant=acme
        → 302 to acme-idp (SAML AuthnRequest)
acme-idp → 302 back with SAMLResponse
gm-console → ada-identity validates signature + assertions
        → mint JWT(RS256) with: sub=user_id, tenant=acme, roles=[Editor]
        → set http-only Secure SameSite=Strict cookie
        → 302 to /tenants (post-login redirect)
```

### Stripe webhook arrives

```
stripe → POST /webhooks/stripe (raw body, Stripe-Signature header)
api-gateway → ada-billing::verify_signature(raw_body, sig) → ok
           → enqueue event (idempotency-keyed by event.id)
           → 200 immediately
worker → load Subscription by customer.id
       → update local state
       → emit audit entry (m11::AuditSink)
```

## 8. Compliance posture (delta vs v0.3.0)

- **GDPR Art. 32**: TLS 1.3 (existing) + WebAuthn bound keys + MFA enforcement.
- **SOC 2 CC6.1**: enforced via casbin + audit_log + tenant RLS.
- **ISO 27001 A.9.4**: passkey preferred, recovery codes, idempotent webhook handler.
- **HIPAA-style least privilege**: ABAC rules ensure `Editor` cannot `Export` another
  tenant's data even if they hold a global Editor role.

## 9. Out of scope (v0.4.0)

- Org-level audit streaming to a customer SIEM (Phase 5).
- Bring-your-own KMS (Phase 5).
- SAML IdP-initiated flow (covered in v0.5.0).
- SCIM provisioning (covered in v0.5.0).