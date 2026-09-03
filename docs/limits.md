# Limits and non-goals

This is a Linux-first MVP. It relies on a `cargo` executable on `PATH`, a
Cargo project that can complete `cargo metadata --format-version 1 --locked`,
and local filesystem access to the workspace evidence.

Known limits:

- The workspace root `.cargo/config.toml` or `.cargo/config` is inspected. The
  tool does not merge every global, parent, or environment-provided Cargo
  configuration source.
- Registry checks use the source information exposed by Cargo metadata and
  checksum fields in the workspace `Cargo.lock`. A missing or unusual source
  is unresolved rather than guessed.
- `forbid_unpinned_git` requires an explicit `rev` field. A tag or branch is
  not treated as an immutable policy pin by this MVP.
- Git pin intent is collected from package manifests that Cargo reports. A
  nonstandard manifest generation mechanism may not be visible.
- Path allowlists use the redacted workspace-relative path or
  `<external>/<name>`. They are not a filesystem authorization boundary.
- Multiple versions are informational. This MVP does not reject duplicate
  versions or implement a version unification policy.
- The JSON schema is versioned at `1` but is not a promise of compatibility
  with future major releases.
- The CLI does not emit SARIF, resolve remote registry aliases, or inspect
  package contents beyond the Cargo evidence needed for these rules.

The tool does not edit manifests, Cargo.lock, Cargo configuration, caches, or
source code. It is not a sandbox and does not make Cargo or a registry safer
than the evidence they provide.
