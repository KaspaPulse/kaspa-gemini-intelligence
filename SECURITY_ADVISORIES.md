# Security Advisories Review Log

This file records intentionally accepted or currently unavoidable RustSec findings in Kaspa Pulse.

## Policy

- Never suppress a RustSec finding without recording why it is accepted and how it will be revisited.
- Treat an unmaintained/transitive warning differently from a proven exploitable vulnerability.
- Remove an exception as soon as the upstream dependency path no longer requires it.
- Re-review the dependency graph whenever Rust, `rusty-kaspa`, SQLx, Teloxide, Reqwest, Axum, Rustls, or other security-sensitive dependencies change.
- CI rejects ignored RustSec IDs that are not documented here and rejects a review date older than 45 days.
- OSV exceptions must be advisory-ID-specific, include a concrete reason, and expire within 45 days; package-wide or ecosystem-wide vulnerability suppression is not permitted.
- A passing OSV scan never replaces the independent `cargo audit` and `cargo deny` gates.

Current CI security gates:

```bash
python3 scripts/check-security-advisories.py --max-age-days 45
cargo audit
cargo deny check
cargo machete
cargo tree --locked -d
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
```

The security workflow also runs the SHA-pinned official OSV Scanner reusable workflow against `Cargo.lock` and applies only the reviewed, time-bounded entries in `osv-scanner.toml`.

Last automated review: **2026-08-08**

---

## Scorecard / OSV aggregation note

OpenSSF Scorecard currently groups eight RustSec records in this dependency graph under its broad `Vulnerabilities` check. That aggregate must not be interpreted as eight equivalent exploitable runtime vulnerabilities.

Seven of the eight records are RustSec `INFO Unmaintained` notices: `RUSTSEC-2025-0052`, `RUSTSEC-2024-0375`, `RUSTSEC-2024-0388`, `RUSTSEC-2024-0384`, `RUSTSEC-2024-0436`, `RUSTSEC-2024-0370`, and `RUSTSEC-2026-0173`. `RUSTSEC-2021-0145` is `INFO Unsound`, affects `atty` on Windows, has no patched release, and is retained only through an upstream/transitive path. Production container validation remains Linux-based.

After dependency-path review, these eight records are encoded in `osv-scanner.toml` as narrow advisory-ID exceptions expiring on **2026-09-22**. CI rejects expired, undocumented, duplicate, malformed, or excessively long-lived OSV exceptions. This is not a blanket package/ecosystem override: new advisories remain scannable, and `cargo audit` plus `cargo deny` remain independent security controls.

The purpose of these OSV exceptions is to encode the reviewed non-actionability of specific upstream/transitive findings while preserving an automatic expiry and re-review requirement; they must not be used to hide a locally actionable vulnerability.

---

## Current managed findings

### RUSTSEC-2024-0388 — `derivative` unmaintained

Status: temporary transitive exception.

Known path includes the upstream Kaspa/Arkworks dependency stack (`kaspa-txscript` → Arkworks components → `derivative`). Kaspa Pulse does not select `derivative` directly.

Action: keep `cargo audit`/`cargo deny` enabled and remove the exception when upstream no longer resolves this crate.

### RUSTSEC-2026-0173 — `proc-macro-error2` unmaintained / future compatibility

Status: temporary build-time transitive exception.

The current resolved graph includes `aquamarine` → `proc-macro-error2`. Rust 1.97.1 also reports this crate in its future-incompatibility output. Kaspa Pulse does not depend on it directly.

Action: track upstream replacement/removal and delete the exception when a safe path exists. Re-evaluate immediately if the advisory changes from maintenance/future compatibility to an exploitable vulnerability.

### RUSTSEC-2024-0320 — `yaml-rust` unmaintained

Status: visible transitive warning.

Kaspa Pulse does not depend on `yaml-rust` directly. The warning remains visible in automated security output rather than being represented as a clean/no-warning state.

Action: monitor the upstream dependency path and remove it when upstream dependencies stop resolving the crate.

### RUSTSEC-2024-0384 — `instant` unmaintained

Status: temporary transitive exception.

Kaspa Pulse does not depend on `instant` directly.

Action: monitor upstream Kaspa/dependency updates and remove the exception when the crate leaves the graph.

### RUSTSEC-2023-0071 — RSA advisory

Status: accepted only while the affected optional database path is not selected by the application.

Kaspa Pulse is PostgreSQL-only and does not select a MySQL application backend. SQLx default features are disabled and the application explicitly enables PostgreSQL.

Action: re-evaluate if MySQL/RSA-backed authentication is ever introduced.

### RUSTSEC-2025-0052 — `async-std` unmaintained

Status: upstream/transitive Kaspa dependency.

Kaspa Pulse application code does not directly select `async-std`.

Action: track `rusty-kaspa` updates and remove the exception once upstream no longer requires the crate.

### RUSTSEC-2024-0375 and RUSTSEC-2021-0145 — `atty`

Status: upstream/transitive exceptions.

Kaspa Pulse does not directly depend on `atty`. `RUSTSEC-2021-0145` is an informational unsoundness advisory specific to Windows and has no patched `atty` release; production container validation is Linux-based.

Action: remove these exceptions when the upstream dependency chain is updated.

### RUSTSEC-2024-0436 — `paste`

Status: upstream/transitive or build-time exception.

Action: keep build-pipeline security checks enabled and remove the exception after the upstream path disappears.

### RUSTSEC-2024-0370 — `proc-macro-error`

Status: upstream/transitive build-time exception.

Action: remove after the upstream dependency path is replaced.

### RUSTSEC-2025-0134 — `rustls-pemfile`

Status: TLS-related transitive exception and therefore treated as security-sensitive.

Action: monitor closely and remove as soon as the resolved upstream TLS stack permits it.

### RUSTSEC-2024-0407 — `linkme`

Status: upstream/transitive exception.

Verified on 2026-08-08: `workflow-rs` 0.19.0 is an upstream maintenance/modernization release that explicitly addresses RustSec advisories, but `rusty-kaspa` `v2.0.1` and its current `master` still declare the `workflow-*` 0.18.x line. Forcing `workflow-*` 0.19 into Kaspa Pulse would cross a pre-1.0 minor compatibility boundary without Kaspa upstream validation, so no local override is applied.

Action: remove this exception when a stable `rusty-kaspa` release adopts a compatible patched workflow dependency path. The weekly `rusty-kaspa` updater will detect the next stable tag and run the full validation gates before opening an update PR.

---

## Git dependency policy

Git dependencies are allowed only for explicitly reviewed sources. Floating branches are not accepted for production dependencies when an immutable release tag/revision is available.

Approved Git source:

```text
https://github.com/kaspanet/rusty-kaspa
```

The current Kaspa SDK dependencies are pinned to the approved `v2.0.1` tag. As verified on 2026-08-08, `v2.0.1` is the newest stable `rusty-kaspa` tag available from the upstream tag list.

---

## Release review checklist

Before a production release:

```bash
python3 scripts/check-security-advisories.py --max-age-days 45
cargo fmt --all -- --check
cargo check --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo audit
cargo deny check
cargo machete
cargo tree --locked -d
cargo build --locked --release --all-features
docker build --pull -t kaspa-pulse:release .
```

Operationally verify local endpoints where enabled:

```bash
curl http://127.0.0.1:18080/healthz
curl http://127.0.0.1:18080/readyz
curl http://127.0.0.1:18080/metrics
```

---

## Secret and repository hygiene

Never commit `.env` files, database dumps, backup archives, generated repository exports, or runtime panic markers. If a real credential was committed, rotate it immediately; deleting the latest file alone does not remove it from Git history.
