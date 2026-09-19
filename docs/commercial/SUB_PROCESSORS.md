# Sub-processors

A sub-processor is a third party that processes Customer Data on behalf of Ada project team
to deliver the gm-console / Ada platform Service.

This page is the canonical list per Privacy Policy §5. Sub-processors are added to this list
at least 30 days before processing starts.

<!-- TODO: source: replace placeholder rows when contracts signed -->

| Sub-processor | Service performed          | Data scope                  | Hosting region | DPA URL                                          |
|---------------|----------------------------|-----------------------------|----------------|--------------------------------------------------|
| Cloudflare    | CDN, DDoS protection       | HTTP headers, request IPs   | global edge    | https://example.cloudflare.com/dpa               |
| AWS           | hosting (k8s control plane)| tenant runtime data         | `ap-northeast-1` | https://aws.amazon.com/service-terms/             |
| GitHub        | source hosting, CI         | source code                 | us-east-1      | https://docs.github.com/en/site-policy-gh        |
| Sentry        | error tracking (opt-out)   | stack traces                | `ap-northeast-1` | https://sentry.io/legal/dpa/                     |
| Postmark      | transactional email        | email addresses             | us-east-1      | https://postmarkapp.com/legal/dpa                |

## How to receive change notices

Account administrators are emailed at least 30 days before any addition, replacement, or
removal. To opt out of any sub-processor that materially affects your deployment, contact
`privacy@kanvas.dev` and we will provide an alternative path or terminate.

## Historical changes

| Date       | Change                                       |
|------------|----------------------------------------------|
| 2026-09-19 | Initial publication (gm-console 0.1.0 scaffold) |
