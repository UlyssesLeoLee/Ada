# App Store / Google Play — Listing Copy (gm-console Mobile)

Source of truth for store metadata. Worker-C will polish to spec; Mavis scaffold here so
shipment can begin from a non-empty template.

## App Name

**gm-console — Ada Platform on the go**

(48 chars; will be reviewed against App Store guidance)

## Tagline

**Run your data pipelines from your phone.**

(40 chars)

## Subtitle (Apple only)

> Pipe · Monitor · Recover

## Promotional Text (Apple, 170 chars)

Real-time visibility into your Ada workflows. Inspect pipeline health, restart stuck jobs,
review audit events, and recover from incidents without touching a terminal. Built for SREs
and operators who need answers when the laptop is closed.

## Short Description (Google Play, 80 chars)

Run, monitor, and recover Ada data pipelines from your phone.

## Full Description

gm-console is the mobile companion to the Ada platform — the open-source, low-code data
pipeline engine used by ops teams who want to ship ETL workflows without hand-coding
plumbing.

With gm-console on iOS or Android you can:

- **Inspect pipelines** — see live status of every acquisition, transformation, and exporter
- **Trigger jobs** — start, pause, or restart tasks when needed
- **Monitor incidents** — receive push notifications for tenant events you care about
- **Audit and recover** — review recent remediations, re-run failed jobs, export run logs
- **Tenant-aware** — switch between the workspaces you operate; RBAC enforced upstream
- **Offline-tolerant** — cached dashboards for the past 24h, full functionality when online

Built for the operators who hold the pager:

- **SREs** responding to incidents from anywhere
- **Data platform owners** running cross-team pipelines
- **Compliance reviewers** auditing activity logs

Under the hood: gm-console talks to the Ada api-gateway; all data stays inside the
deployment you operate. We never store your pipeline data on our servers.

Open source, AGPL v3 + written consent. Read the LICENSE shipped with the app.

## Keywords (Apple, comma-separated)

pipes,etl,monitor,ops,sre,workflow,data,audit,low-code,recovery

## Category

- Apple Primary: **Developer Tools** | Secondary: **Business**
- Google Play: **Developer Tools** (or **Productivity**)

## Support URL

> https://ada.kanvas.dev/support

## Marketing URL (optional)

> https://ada.kanvas.dev

## Privacy Policy URL

> https://ada.kanvas.dev/privacy

## What's New (per release)

Templated in /docs/CHANGELOG.md per crate release; worker-C does final wording.

## In-App Purchases / Subscriptions

> None in scaffold. Paid tiers belong to web SaaS tier — not the mobile app.
