# Research notes

Research pass: 2026-09-03.

## Sources reviewed

- [Cargo source replacement](https://doc.rust-lang.org/cargo/reference/source-replacement.html)
  defines named sources and `replace-with`, and distinguishes replacement
  sources from patching.
- [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html)
  documents `[source]`, `[registries]`, and the relevant local configuration
  shapes.
- [Specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)
  documents registry, git, `rev`, tag, branch, and path dependency forms.
- [Cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)
  documents the machine-readable `--format-version 1` contract.
- [Cargo resolver](https://doc.rust-lang.org/stable/cargo/reference/resolver.html)
  documents that multiple versions can coexist in a resolved graph.
- [Overriding dependencies](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)
  documents root-workspace behavior for `[patch]` and the deprecated
  `[replace]` mechanism.

## Decisions

Cargo metadata is the graph authority. The tool delegates resolution to
`cargo metadata --format-version 1 --locked` rather than implementing a
second resolver. Cargo.lock is read only for checksum evidence, and local
source replacement names are read so the effective registry label can be
checked. Multiple versions are reported as information because Cargo permits
them and the brief does not ask this MVP to reject them.

The output is JSON rather than SARIF. The product has a narrow, local report
model and no need to claim interoperability with a larger findings taxonomy.
The stable schema, deterministic ordering, and exit codes are enough for the
first CI integration.

The explicit `rev` requirement treats branches and tags as non-immutable
policy inputs. This is stricter than Cargo's ability to resolve them and makes
the policy's security intent visible instead of inferring immutability from a
lockfile alone.

## Boundary

This research supports a source-policy compiler, not a package manager,
resolver, vulnerability scanner, license engine, or `cargo-deny` clone. The
implementation stays within those boundaries.
