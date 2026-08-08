## Summary

Describe the problem and the chosen solution.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --locked --all-targets --all-features`
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked --all-targets --all-features`
- [ ] Dependency/security checks completed when applicable
- [ ] Docker build/smoke test completed when container behavior changed

## Security and compatibility

- [ ] No secrets, tokens, production credentials, or private user data are included
- [ ] New GitHub Actions are pinned to immutable commit SHAs
- [ ] Workflow/runtime permissions remain least-privilege
- [ ] Behavior changes include tests or an explanation why tests are not applicable

## Notes

List migrations, operational impact, follow-up work, or reviewer guidance here.
