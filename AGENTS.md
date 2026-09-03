# Local operating contract

This repository is a focused Rust CLI. The source files, tests, workflow, and
release evidence are authoritative.

## Commands

- Build: `cargo build --locked`
- Test: `cargo test --all-targets --locked`
- Format check: `cargo fmt --all -- --check`
- Lint: `cargo clippy --all-targets --all-features --locked -- -D warnings`
- Documentation: `RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked`
- Package check: `cargo package --locked`
- Security audit: `cargo audit`
- Fuzz check: `timeout --foreground 60s env RUSTUP_TOOLCHAIN=nightly cargo fuzz run policy -- -max_total_time=10 -verbosity=0 -print_final_stats=1`

## Scope

Keep changes inside the product brief. `cargo-policy` checks explicit source
rules from Cargo evidence. It does not become a package manager, resolver,
vulnerability scanner, license engine, or `cargo-deny` replacement.

## Verification

Plan the change, define a testable result, make a small edit, read back every
changed file, run the relevant checks, then run the full release gate at
milestones. Record exact output in private `qa/evidence/` files. Do not place
credentials, private project data, or generated build output in the public
repository.

## Release

Follow [`docs/release.md`](docs/release.md). Do not publish or tag a release
until local checks, audit, bounded fuzzing, package inspection, hosted checks,
and independent artifact verification are complete.
