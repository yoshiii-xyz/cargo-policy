# Design

## Evidence flow

```text
policy.toml
    |
    v
strict TOML policy parser
    |
    +--> cargo metadata --format-version 1 --locked
    |        |
    |        +--> resolved packages and workspace members
    |        +--> source classification
    |
    +--> Cargo.lock when checksum evidence is required
    +--> workspace .cargo/config.toml or .cargo/config
    +--> package manifests for explicit git rev intent
    |
    v
sorted report and exit status
```

Cargo remains responsible for dependency resolution. `cargo-policy` consumes
the machine-readable metadata output and does not reproduce Cargo's resolver.
This keeps the MVP focused on policy evaluation and avoids silently diverging
from Cargo's interpretation of a workspace.

## Source classification

- A package in `workspace_members` is `workspace` and is reported without an
  external-source violation.
- A package with no source is `path` and is matched against a redacted,
  workspace-relative path.
- A `registry+` source is `registry`. The crates.io endpoint maps to the
  `crates.io` label unless Cargo source replacement gives it another effective
  name. Named registry indexes use the local `[registries]` mapping when it is
  present.
- A `git+` source is `git`. The URL is normalized to a host and repository
  reference. An explicit nonempty `rev` in a dependency declaration is the
  pinning evidence required by `forbid_unpinned_git`.
- Any other source is unresolved. The tool does not guess.

## Policy compilation

`compile` validates the policy and emits five ordered rules:

1. registry allowlist;
2. git allowlist;
3. path allowlist;
4. registry checksum requirement;
5. git revision requirement.

Compilation is intentionally a readable rule description, not a new package
resolution language. `check` evaluates the same policy against Cargo's
resolved graph.

## Diagnostics

Diagnostics have a severity, stable code, optional package, and detail. The
report sorts packages by name, version, source kind, and source reference. It
sorts diagnostics by severity, code, package, and detail. Informational
multiple-version and source-replacement records do not fail a check. An
unresolved source or failed evidence read has a distinct unresolved status.

## Privacy boundary

Absolute workspace roots are redacted in output. Path dependencies outside
the workspace retain only their final directory name. Source URLs are policy
inputs and are retained in normalized form so a human can audit the decision.
No report is written automatically.
