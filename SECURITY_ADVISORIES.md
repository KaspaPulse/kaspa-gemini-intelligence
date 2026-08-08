# Security Advisories Review Log

This file records intentionally accepted or currently unavoidable RustSec findings in Kaspa Pulse.

## Policy

- Never suppress a RustSec finding without recording why it is accepted and how it will be revisited.
- Treat an unmaintained/transitive warning differently from a proven exploitable vulnerability.
- Remove an exception as soon as the upstream dependency path no longer requires it.
- Re-review the dependency graph whenever Rust, `rusty-kaspa`, SQLx, Teloxide, Reqwest, Axum, Rustls, or other security-sensitive dependencies change.

Current CI security gates:

```bash
cargo audit
cargo deny check
cargo machete
cargo tree --locked -d
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
```

Last automated review: **2026-08-08**

---

## Scorecard / OSV aggregation note

OpenSSF Scorecard currently groups eight RustSec records in this dependency graph under its broad `Vulnerabilities` check. That aggregate must not be interpreted as eight equivalent exploitable runtime vulnerabilities.

Seven of the eight records are RustSec `INFO Unmaintained` notices: `RUSTSEC-2025-0052`, `RUSTSEC-2024-0375`, `RUSTSEC-2024-0388`, `RUSTSEC-2024-0384`, `RUSTSEC-2024-0436`, `RUSTSEC-2024-0370`, and `RUSTSEC-2026-0173`. `RUSTSEC-2021-0145` is `INFO Unsound`, affects `atty` on Windows, has no patched release, and is retained only through an upstream/transitive path. Production container validation remains Linux-based.

These records remain visible and monitored; they are not represented as a zero-advisory state. No additional ignore is added merely to improve an OpenSSF Scorecard number.

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

Action: remove when the upstream dependency path is patched or removed.

---

## Git dependency policy

Git dependencies are allowed only for explicitly reviewed sources. Floating branches are not accepted for production dependencies when an immutable release tag/revision is available.

Approved Git source:

```text
https://github.com/kaspanet/rusty-kaspa
```

The current Kaspa SDK dependencies are pinned to the approved `v2.0.1` tag.

---

## Release review checklist

Before a production release:

```bash
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
