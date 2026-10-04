# ada-identity

> Identity, authentication, MFA, JWT minting for Ada v0.4.0.

## What this crate provides

| Module       | Purpose                                                                |
|--------------|------------------------------------------------------------------------|
| `config`     | env-driven configuration (no values printed)                           |
| `oidc`       | OpenID Connect Authorization Code + PKCE RP (token exchange not implemented) |
| `saml`       | SAML 2.0 SP (AuthnRequest + assertion parsing; validation not implemented)   |
| `webauthn`   | WebAuthn / FIDO2 RP                                                    |
| `passkey`    | WebAuthn resident-key flow                                             |
| `totp`       | RFC 6238 TOTP                                                          |
| `recovery`   | 10 single-use 8-char recovery codes                                    |
| `session`    | opaque session token + cookie                                          |
| `mint`       | JWT minting / verification (RS256) — both fail closed                  |
| `jwks`       | JWKS endpoint payload                                                  |
| `rate_limit` | token-bucket limiter for `/login`                                      |

## Fail-closed boundaries

Three entry points return `Err` for every input rather than a
plausible-looking success, because the validation they stand in for is not
implemented yet:

- `mint::mint_jwt` — no RS256 signer. Returns
  `IdentityError::JwtSigningUnavailable`. It does not emit a token with an
  empty signature segment.
- `mint::verify_jwt_stub` — same cause, so it reuses that variant. It does
  not decode a payload and call it verified.
- `saml::parse_response` — no assertion validation. Returns
  `IdentityError::Saml`.
- `oidc::complete_flow` — no token-endpoint call. Returns
  `IdentityError::Oidc`. `begin_flow` still returns real session material.

An `Err` here is a missing-feature signal, not a transient failure: do not
retry it, and do not substitute a fallback that treats "no error" as
"authenticated".

## Environment

| Variable                       | Required | Description                                  |
|--------------------------------|----------|----------------------------------------------|
| `IDENTITY_JWT_PRIVATE_KEY`     | yes      | PEM-encoded RSA private key                  |
| `IDENTITY_JWT_KID`             | yes      | Key ID (rotation via JWKS `kid` rollover)    |
| `IDENTITY_BASE_URL`            | yes      | `https://gm-console.kanvas.dev`              |
| `IDENTITY_RP_ORIGIN`           | optional | Default `https://gm-console.kanvas.dev`      |
| `IDENTITY_TRUSTED_PROXIES`     | optional | Comma-separated CIDR list                    |
| `IDENTITY_RATE_LIMIT_PER_MIN`  | optional | Default 10                                   |

No env var values are logged (per memory 2026-08-27 hard ban).

## Out of scope (v0.5.0+)

- Live IdP integration tests (the v0.4.0 tests are in-process).
- SAML IdP-initiated flow.
- SCIM provisioning.