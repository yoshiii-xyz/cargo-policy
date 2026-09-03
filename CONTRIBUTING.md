# Contributing

Keep changes narrow and tied to the dependency-source policy MVP. Do not add
frontend code, hosted services, cloud backends, package resolution, or a
second policy system.

Before opening a pull request, run:

```text
git diff --check
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked
cargo package --locked
```

If dependency or workflow files change, run the relevant audit and inspect
the package contents. Add or update tests for every changed policy rule.
Keep diagnostics deterministic and avoid absolute paths or secret values in
fixtures and logs.

Do not commit credentials, registry login files, private audit evidence, or
generated `target/` output. A human reviews and commits accepted changes.
