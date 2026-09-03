# Security policy

## Scope

This repository contains a local Rust CLI. The supported security scope is
the code in this repository and the published `cargo-policy` package.

## Reporting a vulnerability

Do not open a public issue for an unpatched vulnerability. Use the private
GitHub security advisory flow for `joshiii-xyz/cargo-policy` when available.
If that flow is unavailable, contact the repository owner through the GitHub
profile and include a minimal reproduction, affected version, and impact.
Do not include secrets or private project files in a report.

## Security design

The CLI runs local `cargo metadata --locked` and reads only the policy,
workspace metadata, lockfile, package manifests, and local Cargo source
configuration needed for its checks. It does not execute package build
scripts itself, contact a remote service, or write project files.

Reports redact absolute workspace paths and external path prefixes. Policy
files should still be treated as project configuration and should not contain
credentials.

The project pins GitHub Action references to reviewed commit SHAs. Dependency
audit output is part of the release evidence. A passing audit is not a claim
that all transitive software is free of risk.
