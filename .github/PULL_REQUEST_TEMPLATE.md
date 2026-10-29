## What changed and why

## How it was checked

- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- [ ] Commands quoted in docs were run and the output pasted
- [ ] `python3 docs/check-links.py`
- [ ] New behaviour has a test; anything not built is marked "proposed"
- [ ] Examples and fixtures are synthetic
- [ ] [CHANGELOG.md](../CHANGELOG.md) updated for user-visible changes

## Security

Does this change how embedded document text, ZIP entries or paths are handled? If yes, describe the
threat it addresses or could introduce.
