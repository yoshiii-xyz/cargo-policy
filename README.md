# cargo-policy

`cargo-policy` compiles a small, explicit dependency-source policy into a
deterministic check over a Cargo project's resolved dependency graph.

Status: MVP release candidate. The exact release state is recorded in
`docs/release.md`.

## Install

After the first crates.io release:

```text
cargo install cargo-policy --locked
```

From this repository:

```text
cargo run --locked -- check --policy policy.toml
```

## Quick start

Create `policy.toml`:

```toml
allow_registry = ["crates.io", "corp"]
allow_git = ["github.com/org/*"]
allow_path = ["vendor/*"]
require_checksum = true
forbid_unpinned_git = true
```

Run the check from the Cargo workspace root:

```text
cargo-policy check --policy policy.toml
cargo-policy explain serde --policy policy.toml
cargo-policy compile --policy policy.toml --format json
```

`check` invokes `cargo metadata --format-version 1 --locked`, reads the
workspace `Cargo.lock` when checksum evidence is required, and reports the
source classification for every resolved package. It does not resolve
versions, download packages, edit manifests, or replace Cargo.

## Policy

The policy file is TOML with five fields:

| Field | Meaning |
| --- | --- |
| `allow_registry` | Registry labels or wildcard patterns allowed for external packages. |
| `allow_git` | Normalized git host and repository patterns allowed for git packages. |
| `allow_path` | Workspace-relative path patterns allowed for external path packages. |
| `require_checksum` | Require a checksum entry for every registry package in `Cargo.lock`. |
| `forbid_unpinned_git` | Require an explicit nonempty `rev` in git dependency declarations. |

An empty allowlist denies that source category for external dependencies.
Workspace members are reported but are not subject to external source
allowlists. `*` matches any sequence of characters and `?` matches one
character. Unknown policy fields and invalid patterns are errors.

Cargo source replacements in the workspace `.cargo/config.toml` or legacy
`.cargo/config` are reported. A replacement such as `crates-io` to `corp`
changes the effective registry label used by the allowlist check.

## Output and exit codes

Text output is intended for local inspection. JSON output is a stable report
with `schema_version: 1`, sorted packages, and sorted diagnostics:

```text
cargo-policy check --policy policy.toml --format json
cargo-policy explain serde --policy policy.toml --format json
```

Exit codes are:

| Code | Meaning |
| --- | --- |
| `0` | The policy passed. Informational diagnostics may be present. |
| `2` | The policy found one or more actionable violations. |
| `3` | Evidence could not be resolved, input was malformed, Cargo failed, or a limit was reached. |

JSON is the supported machine-readable format. SARIF is intentionally not
implemented because this MVP has a small, local report schema and does not
need a second diagnostic transport.

## Safety and data handling

The tool reads the policy file, Cargo metadata, the workspace lockfile when
needed, package manifests for git `rev` intent, and the workspace Cargo
source configuration. It does not transmit project contents, retain reports,
capture credentials, or print absolute workspace paths in reports. External
path names are reduced to `<external>/<name>` and the workspace name is
reduced to `<absolute>/<name>`.

The check runs Cargo as a child process with `--locked`. Cargo may use its
normal local cache and configured credentials, but `cargo-policy` does not
read or emit credential files.

## Limits and non-goals

See [`docs/limits.md`](docs/limits.md). This project is not a package manager,
dependency resolver, vulnerability database, license policy engine, lockfile
rewriter, or copy of `cargo-deny`. It supports the Cargo metadata and local
configuration shapes needed by this MVP on Linux. It does not promise full
coverage of every Cargo source or configuration feature.

## Testing and development

```text
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps --locked
cargo package --locked
cargo audit
```

The bounded fuzz command and release evidence are documented in
[`docs/release.md`](docs/release.md). Contribution rules are in
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

MIT. See [`LICENSE`](LICENSE).
