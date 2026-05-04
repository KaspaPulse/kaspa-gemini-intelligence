# Security Advisories Review Log

This file documents every intentionally ignored RustSec advisory in this project.

## Policy

- Do not ignore a RustSec advisory without documenting source, reachability, risk, mitigation, and review date.
- Re-check this file when updating Kaspa SDK, tokio, sqlx, reqwest, teloxide, axum, rustls, or any dependency that touches networking, cryptography, persistence, Telegram input handling, or build-time macros.
- CI must run:
  - `cargo audit`
  - `cargo deny check`
  - `cargo tree -d`
  - `cargo machete`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
- Ignored advisories are allowed only when:
  - the vulnerable code path is not reachable,
  - the dependency is an upstream transitive dependency with no fixed version yet,
  - the affected feature is disabled,
  - or the dependency is build-time only and the risk is documented.

Last automated review date: 2026-05-04

---

## RUSTSEC-2023-0071

Source: RSA dependency pulled by optional/removed database feature path.

Reachability: Not directly used by this bot. Project uses PostgreSQL through `sqlx`, not MySQL.

Risk: Low, assuming MySQL feature remains removed/disabled and no code path reintroduces RSA-based MySQL authentication.

Mitigation:

- Keep PostgreSQL-only database path.
- Keep `cargo audit` enabled.
- Re-check after dependency updates.

Review date: 2026-06-01

Upstream issue: N/A unless dependency path becomes reachable again.

---

## RUSTSEC-2025-0052

Source: `async-std`, currently treated as an upstream/transitive Kaspa SDK dependency.

Reachability: Not directly used by the bot application code.

Risk: Medium because it is an unmaintained runtime dependency, even if transitive.

Mitigation:

- Keep monitoring Kaspa SDK updates.
- Prefer removal when upstream Kaspa SDK stops pulling it.
- CI runs `cargo audit` and `cargo deny check`.

Review date: 2026-06-01

Upstream issue: Track through Kaspa SDK dependency updates.

---

## RUSTSEC-2024-0375

Source: `atty`, upstream/transitive dependency.

Reachability: Not directly used by the bot application code.

Risk: Low to Medium depending on whether the transitive path is reachable at runtime.

Mitigation:

- Keep advisory visible in this document.
- Re-check after Kaspa SDK and CLI-related dependency updates.

Review date: 2026-06-01

Upstream issue: Track through upstream dependency chain.

---

## RUSTSEC-2021-0145

Source: `atty` soundness advisory, upstream/transitive dependency.

Reachability: Not directly used by the bot application code.

Risk: Low to Medium.

Mitigation:

- Monitor with `cargo audit`.
- Remove ignore when dependency chain no longer includes affected crate.

Review date: 2026-06-01

Upstream issue: Track through upstream dependency chain.

---

## RUSTSEC-2024-0384

Source: `instant`, upstream/transitive dependency.

Reachability: Not directly used by the bot application code.

Risk: Low, assuming no direct runtime reliance.

Mitigation:

- Monitor with `cargo audit` and `cargo deny check`.
- Remove ignore when upstream dependency is removed.

Review date: 2026-06-01

Upstream issue: Track through upstream dependency chain.

---

## RUSTSEC-2024-0436

Source: `paste`, upstream/transitive/build dependency.

Reachability: Not directly used by application runtime.

Risk: Low to Medium. Procedural macro dependencies can matter during build-time supply chain review.

Mitigation:

- Keep CI dependency checks enabled.
- Remove ignore after upstream update.

Review date: 2026-06-01

Upstream issue: Track through upstream dependency chain.

---

## RUSTSEC-2024-0370

Source: `proc-macro-error`, upstream/transitive/build dependency.

Reachability: Build-time/transitive dependency. Not directly used by application runtime.

Risk: Low to Medium.

Mitigation:

- Keep build pipeline dependency checks enabled.
- Remove ignore after upstream update.

Review date: 2026-06-01

Upstream issue: Track through upstream dependency chain.

---

## RUSTSEC-2025-0134

Source: `rustls-pemfile`, upstream/transitive dependency.

Reachability: Potentially relevant to TLS/certificate parsing depending on dependency path.

Risk: Medium because TLS-related dependencies are security-sensitive.

Mitigation:

- Monitor closely.
- Prefer upgrading transitive dependency when upstream allows.
- Keep `reqwest` configured with `rustls-tls`.
- Re-check after Kaspa SDK or TLS dependency updates.

Review date: 2026-06-01

Upstream issue: Track through upstream dependency chain.

---

## RUSTSEC-2024-0407

Source: `linkme`, upstream/transitive dependency.

Reachability: Not directly used by the bot application code.

Risk: Low to Medium.

Mitigation:

- Monitor with `cargo audit`.
- Remove ignore when upstream dependency is removed or patched.

Review date: 2026-06-01

Upstream issue: Track through upstream dependency chain.

---

## Git Dependency Policy

Current project policy:

- Git dependencies are allowed only for explicitly approved sources.
- Approved Git sources must be listed in `deny.toml`.
- Prefer tags or immutable revisions over floating branches.
- Re-run `cargo update`, `cargo audit`, and `cargo deny check` after updating Git dependencies.

Approved Git sources:

- https://github.com/kaspanet/rusty-kaspa
- https://github.com/murar8/serde_nested_with

Review date: 2026-06-01

---

## Operational Security Notes

The following files must never be committed:

```text
.env
.env.*
.backup/
backups/
*.dump
*.sql.dump
*.bak
repo-before-history-clean-*.bundle
project_code_export_*.txt
````

If any of these were committed:

1. remove them from the current repository state
2. rewrite Git history
3. force-push protected branches intentionally
4. rotate affected credentials
5. reset existing server clones with `git reset --hard origin/dev`

---

## Review Checklist

Before production deployment:

```bash
cargo fmt
cargo audit
cargo deny check
cargo tree -d
cargo machete
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Also verify local-only endpoints:

```bash
curl http://127.0.0.1:18080/healthz
curl http://127.0.0.1:18080/readyz
curl http://127.0.0.1:18080/metrics
```

