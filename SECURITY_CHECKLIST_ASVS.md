# Kaspa Pulse Security Checklist - OWASP ASVS Inspired

This is a project-specific checklist for Telegram bot and Rust service security.

## Admin Authorization

- [ ] ADMIN_ID is required in production.
- [ ] Sensitive admin actions require confirmation.
- [ ] Confirmation tokens expire quickly.
- [ ] Admin actions are recorded in admin_audit_log.

## Input Validation

- [ ] Wallet input uses Kaspa address validation.
- [ ] Raw messages are length-limited.
- [ ] Wallet messages reject invisible/control characters.
- [ ] Multi-wallet paste is rejected unless explicitly supported.

## Output Encoding

- [ ] User-controlled text is escaped before Telegram HTML parse mode.
- [ ] Logs mask wallet, TXID, and block hash values.

## Webhook Security

- [ ] WEBHOOK_SECRET_TOKEN is 32+ characters.
- [ ] WEBHOOK_BIND is 127.0.0.1 behind reverse proxy.
- [ ] Health and metrics endpoints are local-only.
- [ ] Public reverse proxy blocks /metrics, /healthz, and /readyz.

## Database Security

- [ ] Runtime uses kaspa_pulse_app, not postgres.
- [ ] Migrations are applied through migration files.
- [ ] Runtime schema ensure is disabled in production.
- [ ] Credentials are rotated after any exposure.

## Supply Chain

- [ ] cargo audit passes or advisories are documented.
- [ ] cargo deny check passes.
- [ ] cargo clippy with -D warnings passes.
- [ ] cargo machete passes.
- [ ] cargo test passes.
- [ ] Secret scan passes.
