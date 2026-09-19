# ada-identity

> Identity, authentication, MFA, JWT minting for Ada v0.4.0.

## What this crate provides

| Module       | Purpose                                                                |
|--------------|------------------------------------------------------------------------|
| `config`     | env-driven configuration (no values printed)                           |
| `oidc`       | OpenID Connect Authorization Code + PKCE RP                            |
| `saml`       | SAML 2.0 SP (AuthnRequest + assertion parsing)                         |
| `webauthn`   | WebAuthn / FIDO2 RP                                                    |
| `passkey`    | WebAuthn resident-key flow                                             |
| `totp`       | RFC 6238 TOTP                                                          |
| `recovery`   | 10 single-use 8-char recovery codes                                    |
| `session`    | opaque session token + cookie                                          |
| `mint`       | JWT minting (RS256) + verification stub                                |
| `jwks`       | JWKS endpoint payload                                                  |
| `rate_limit` | token-bucket limiter for `/login`                                      |

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