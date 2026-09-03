# Operations

## Local check

Run the check from the workspace root or pass an explicit manifest path:

```text
cargo-policy check --policy policy.toml
cargo-policy check --policy policy.toml --manifest-path path/to/Cargo.toml --format json
```

The command runs Cargo with `--locked`. A successful Cargo invocation is
required before any package policy decision is made. The check does not write
to the project.

## Interpreting status

- `ok`: no unresolved evidence or policy violation was found.
- `violation`: at least one source, checksum, or git pin rule failed.
- `unresolved`: Cargo, metadata, lockfile, configuration, or source evidence
  could not be interpreted safely.

Use the stable diagnostic code in automation. Human-readable detail may grow
within a schema version.

## CI integration

Keep the policy file in the same repository as the Cargo workspace. A minimal
step is:

```yaml
- name: Check dependency source policy
  run: cargo-policy check --policy policy.toml --format json
```

The command's exit code is the gate. Store the JSON as a CI artifact only when
the repository's retention policy permits it. Reports can contain package
names and normalized source URLs.

## Failure handling

Exit code `2` means the evidence was understood and the policy failed. Exit
code `3` means the result is not safe to treat as a pass. Investigate the
printed error or JSON report, correct the project evidence or policy, and run
the bounded validation commands in `docs/release.md`.
