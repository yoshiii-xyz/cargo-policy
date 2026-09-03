# Product brief

## Mission

Compile a small, explicit dependency-source policy into checks that Cargo
projects and CI can execute.

## MVP commands

```text
cargo-policy check --policy policy.toml
cargo-policy explain PACKAGE --policy policy.toml
cargo-policy compile --policy policy.toml --format json
```

## Policy surface

The MVP accepts registry, git, and path allowlists plus checksum and explicit
git revision requirements. It uses Cargo metadata as the dependency graph
authority and reads Cargo.lock and local source replacement configuration as
supporting evidence.

## Success criteria

- A valid policy compiles to a deterministic JSON document.
- A Cargo workspace can be checked without changing its manifests or lockfile.
- Registry, git, path, checksum, replacement, and multiple-version cases are
  visible in tests and diagnostics.
- Malformed policy and unresolved Cargo evidence produce nonzero failure
  states with actionable details.
- The public package can be installed and run without project-local source.

## Non-goals

The MVP is not a package manager, dependency resolver, vulnerability or
license database, lockfile editor, remote policy service, or full replacement
for `cargo-deny`.
