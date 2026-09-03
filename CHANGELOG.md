# Changelog

## 0.1.0

Initial MVP release candidate.

- Compiles five explicit source policy rules.
- Checks registry, git, path, workspace, and unknown package sources from
  Cargo metadata.
- Reports source replacements, missing checksums, unpinned git revisions, and
  multiple resolved versions.
- Provides deterministic text and JSON output with stable exit codes.
- Includes malformed-input tests, CLI tests, package checks, audit checks, and
  a bounded policy parser fuzz target.
