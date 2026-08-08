# Security Policy

## Supported versions

Security fixes are applied to the current `main` branch and the latest maintained release. Older releases may not receive backported fixes.

## Reporting a vulnerability

Do not open a public issue for an unpatched vulnerability.

When GitHub private vulnerability reporting is enabled for this repository, use the repository's **Security → Report a vulnerability** / Security Advisories flow. If that control is not available, contact the repository owner privately through an existing trusted contact channel before public disclosure.

Include, when possible:

- the affected version or commit;
- the impact and realistic attack scenario;
- a minimal reproduction or proof of concept;
- any suggested mitigation.

Reports should be validated privately, remediation coordinated, and a security advisory published when appropriate.

## Security automation

Dependency-sensitive changes are checked with `cargo audit`, `cargo deny`, and `cargo machete`; Rust application changes also pass formatting, locked compilation, strict Clippy, and tests. Accepted upstream or transitive exceptions are documented in `SECURITY_ADVISORIES.md` and configured in `.cargo/audit.toml` / `deny.toml` as applicable.
