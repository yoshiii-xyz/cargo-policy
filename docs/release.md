# Release

The release target is `0.1.0`. Publication is allowed only after the local,
security, package, fuzz, hosted, and independent artifact checks below have
passed and their exact output has been recorded in private
`qa/evidence/iteration-YYYY-MM-DD.md`.

## Local release gate

```text
git diff --check
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked
cargo package --locked
cargo publish --dry-run --locked
cargo audit
cargo audit --file fuzz/Cargo.lock
```

Run the bounded fuzz target with a hard outer timeout and a 10 second libFuzzer
budget:

```text
timeout --foreground 60s env RUSTUP_TOOLCHAIN=nightly cargo fuzz run policy -- -max_total_time=10 -verbosity=0 -print_final_stats=1
```

The predicate is a zero exit status with no crash artifact before either the
10 second fuzz budget or the 60 second outer timeout. Do not extend the
timeout in an automated retry loop.

Inspect the `.crate` file produced by `cargo package` and confirm that it
contains source, tests, documentation, and workflow files only as intended.

## Publication order

1. Review the diff and local evidence.
2. Commit the release changes on `main`.
3. Push `main`.
4. Publish the crate with `cargo publish --locked`.
5. Verify the crates.io API and docs.rs entry.
6. Push annotated tag `v0.1.0`.
7. Wait for CI, security, CodeQL, and tag package workflows to pass.
8. Download the published crate independently, compare its SHA256 with the
   local package archive, install it in a fresh temporary target, and run the
   JSON smoke check.
9. Record commit, tag, release, workflow URLs, package hash, test counts, and
   validation limits in private evidence.

## Support status

The release is an MVP. It supports the documented local Cargo evidence model
on Linux. The limits in [`limits.md`](limits.md) remain part of the release
contract.
