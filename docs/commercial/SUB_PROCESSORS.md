# Sub-processors

> **DRAFT — NOT PUBLISHED.** The list is split into two tables because the
> previous version conflated them, and the split is the whole problem: it
> named five processors that this repository does not integrate, while
> omitting the three that its code can reach.
>
> Searched across `crates/*/src`, `apps/*/lib`, `deploy/` and the compose
> files: **Cloudflare, AWS, Sentry and Postmark appear nowhere.** No
> dependency, no configuration, no code path, no image. The one place
> Cloudflare was mentioned was a sentence in this banner claiming it
> appeared in a CORS allow-list, and the allow-list contains only
> `gm-console.kanvas.dev`, `staging.gm-console.kanvas.dev` and
> `http://localhost:8080` (`crates/gm-console/src/config.rs`,
> `deploy/k8s/gm-console.yaml`). That correction notice was the one part
> of the file a reader would check, and it cited an artifact that does not
> exist.
>
> The Flutter client ships no crash-reporting SDK at all, so the Sentry row
> described a data flow that does not exist. Do not publish this list, and
> do not use it to answer a customer's data-processing question, until each
> row in the integrated table is backed by a signed DPA and each row in the
> not-integrated table has either been signed or deleted.

A sub-processor is a third party that processes Customer Data on behalf of Ada project team
to deliver the gm-console / Ada platform Service.

This page is the canonical list per Privacy Policy §5. Sub-processors are added to this list
at least 30 days before processing starts.

<!-- gate:integrated -->

## Integrated in this repository

These are referenced by shipped code or by a shipped manifest. "Conditional" means the
code path exists but nothing in `deploy/k8s/` deploys the service, so no data leaves the
cluster unless an operator deploys it.

| Sub-processor | Service performed | Data scope | Where it appears | Status |
|---|---|---|---|---|
| Stripe | payment processing, hosted Billing Portal | tenant + customer identifiers, plan and price IDs, subscription and invoice webhook payloads | `crates/ada-billing`; base URL is the hard-coded constant `DEFAULT_STRIPE_BASE_URL = "https://api.stripe.com/v1"` (`crates/ada-billing/src/config.rs`) | conditional — `ada-billing` is **not** in `deploy/k8s/`; `docs/commercial/v0.5.0-roadmap.md` §2 scopes live calls to a sandbox E2E test that skips without `STRIPE_SECRET_KEY` |
| PagerDuty | incident paging | alert labels, service name, severity | `crates/ada-remediation/src/executor.rs` POSTs to `https://events.pagerduty.com/v2/enqueue` | conditional on `PAGERDUTY_ROUTING_KEY`, declared as a `PLACEHOLDER_…` in `deploy/k8s/ada-remediation.yaml` |
| Slack | incident notification | alert channel and message | `crates/ada-remediation/src/executor.rs` posts to `SLACK_WEBHOOK_URL`; three of the five runbooks in `config/remediation/` carry a `notify_slack` step | conditional on `SLACK_WEBHOOK_URL`, declared as a `PLACEHOLDER_…` in `deploy/k8s/ada-remediation.yaml` |

`crates/gm-console/src/routes.rs` serves a `source_url` pointing at
`github.com/UlyssesLeoLee/ada/blob/main/LICENSE`. That is a link rendered into a
response, not a call to GitHub, so GitHub is not listed as a sub-processor for it.

<!-- end -->

<!-- gate:not-integrated -->

## Not integrated

Placeholder rows carried over from an earlier draft. None is wired up; each is kept so the
gap is visible rather than silently closed, and each should be deleted when it is either
signed or dropped from the architecture.

| Sub-processor | Service claimed | Data scope claimed | Hosting region | Why it is here |
|---|---|---|---|---|
| Cloudflare | CDN, DDoS protection | HTTP headers, request IPs | global edge | no integration; `deploy/web/DEMO_URL.md` mentions "Cloudflare-managed TLS" as the intended deployment model, which is a plan rather than a configuration |
| AWS | hosting (k8s control plane) | tenant runtime data | `ap-northeast-1` | plausible for a hosted deployment; no cluster, account or region is configured by this repository |
| GitHub | source hosting, CI | source code | us-east-1 | the repository is hosted on GitHub and CI runs on GitHub Actions, but no customer data reaches either — this is a source-hosting fact, not a Customer Data disclosure |
| Sentry | error tracking (opt-out) | stack traces | `ap-northeast-1` | no integration. `apps/gm-console-app/pubspec.yaml` has no sentry, firebase or analytics dependency and `lib/` has no telemetry code |
| Postmark | transactional email | email addresses | us-east-1 | no integration; no SMTP or mail SDK in any `Cargo.toml` |

<!-- end -->

## How to receive change notices

Account administrators are emailed at least 30 days before any addition, replacement, or
removal. To opt out of any sub-processor that materially affects your deployment, contact
`privacy@kanvas.dev` and we will provide an alternative path or terminate.

> That address is **unconfirmed** — `docs/commercial/COMPLIANCE_SELFCHECK.md` records it as
> not published anywhere in the repository. Publishing it here as the opt-out channel is a
> claim about a mailbox nobody has verified.

## Historical changes

| Date       | Change                                       |
|------------|----------------------------------------------|
| 2026-09-19 | Initial publication (gm-console 0.1.0 scaffold) |
| see below | Split into integrated / not integrated; Stripe, PagerDuty and Slack added; the banner's CORS-allow-list justification removed |
