# Press Kit — gm-console / Ada Platform

> One-page summary for press, analysts, partnerships. Replace placeholder URLs and logos with
> the hosted brand kit when ready.

> **DRAFT — NOT FOR DISTRIBUTION.** Every `<TODO: ...>` below is an unfilled slot, not
> a placeholder waiting for polish. The mobile platforms cannot be built from this
> repository yet (the tracked tree has no `build.gradle`, `AndroidManifest.xml`,
> `Info.plist` or `Runner.xcodeproj`), and no hosted asset URLs exist. See
> `docs/commercial/COMPLIANCE_SELFCHECK.md`.

## What is Ada?

Ada is an open-source, low-code data pipeline platform. It lets ops teams design ETL
workflows visually (Bevy WASM canvas), run them across multi-tenant infrastructure with
strong isolation, and recover from incidents with built-in audit and remediation.

gm-console is the user-facing Web + iOS + Android operations console for Ada.

## At a glance

- **Founded**: 2026 (project inception)
- **License**: AGPL v3 + WRITTEN-CONSENT (commercial alternative available)
- **Stack**: Rust (24 crates under `crates/`) + Bevy 0.14 WASM frontend + PostgreSQL 16 with RLS
- **Platforms**: Linux, macOS, Windows (CI builds and runs on ubuntu-latest and
  windows-latest). iOS/iPadOS/Android are **not shippable from this repository yet** —
  `apps/gm-console-app` contains Dart sources but no platform project, so
  `mobile-build.yml` cannot produce an IPA or AAB from a clean checkout.
- **Standards**: IPA SLCP-JCF2018 governance baseline; GDPR / PIPL / CCPA privacy posture

## Key features (one-liners)

- **Visual pipeline design** — drag-and-drop nodes on a canvas (web), or trigger from
  phone (mobile).
- **Multi-tenant by design** — PostgreSQL Row Level Security + tenant middleware layer.
- **Operator-first UX** — incidents feed, audit timeline, one-tap recovery.
- **Plugin SDK** — build custom nodes in Rust, sandboxed.

## Mini-FAQ

**Q: Is Ada a SaaS?**
A: Ada ships as open-source; you self-host on Kubernetes. A managed offering is roadmap.

**Q: How does Ada compare to Apache Airflow?**
A: Ada targets a different operator — visual design first, strong tenant isolation, and
built-in mobile ops, rather than script-heavy DAGs.

**Q: What's the license?**
A: AGPL v3 with an additional written-consent addendum for proprietary embedding.
Commercial licenses are negotiated case-by-case (licensing@kanvas.dev).

**Q: When did gm-console Mobile launch?**
A: It has not. The mobile client is Dart source only; the iOS and
Android platform projects are not in the repository, so no build has
been produced or submitted. Any date here would be a guess.

## Brand assets

- **Wordmark**: not hosted — no URL exists yet
- **Logomark**: source SVG is at `apps/gm-console-app/brand/icon-source.svg`;
  no hosted or packaged variant exists
- **Color**: see [`brand/TOKENS.md`](brand/TOKENS.md) (this one is real)
- **Press image pack**: does not exist

## Contact

The addresses below appear only in this directory. Nothing in the
repository publishes them — there is no `SECURITY.md`, no
`security.txt`, and no contact route — so whether a given mailbox is
monitored cannot be established from here. Confirm before publishing
any of them.

- Press & media: `press@kanvas.dev` (unconfirmed)
- Partnerships: `partnerships@kanvas.dev` (unconfirmed)
- Legal & licensing: `licensing@kanvas.dev` (unconfirmed)
- Privacy: `privacy@kanvas.dev` (unconfirmed)

## Quotes on file

**None.** No customer has been approached for a quote, and no consent
exists for one to be published.

This section previously carried a fabricated testimonial — *"Ada gave
our operations team one source of truth for 40+ business-critical
pipelines."* — attributed to `<!-- TODO: customer name -->`. A quote
attributed to nobody is not an unfinished draft; it is an invented
endorsement, and anything that quotes this kit would publish it. It has
been removed rather than left for someone to fill in.

## Boilerplate (≤ 50 words)

Ada is the open-source low-code data pipeline platform built for SREs and data platform
owners. With Ada, teams design ETL workflows visually, run them in multi-tenant
infrastructure, and operate them from the web console. AGPL v3.
