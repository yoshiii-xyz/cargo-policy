use std::{fs, path::PathBuf, process::Command};

use cargo_policy::{Policy, check, compile_policy, explain, render_json};

#[test]
fn checks_this_project_with_a_registry_checksum_policy() {
    let policy_path = write_policy("allow_registry = [\"crates.io\"]\nrequire_checksum = true\n");
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let report = check(&policy_path, Some(&manifest_path)).expect("cargo metadata report");

    assert_eq!(report.status, "ok");
    assert!(report.packages.iter().any(|package| package.name == "toml"));
    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != "error")
    );
}

#[test]
fn explains_a_resolved_package() {
    let policy_path = write_policy("allow_registry = [\"crates.io\"]\n");
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let report = explain(&policy_path, "serde", Some(&manifest_path)).expect("explain report");

    assert_eq!(report.package_query, "serde");
    assert!(!report.matches.is_empty());
}

#[test]
fn compile_and_cli_json_are_deterministic() {
    let policy_path = write_policy(
        "allow_registry = [\"crates.io\"]\nallow_git = [\"github.com/org/*\"]\nallow_path = [\"vendor/*\"]\nrequire_checksum = true\nforbid_unpinned_git = true\n",
    );
    let compiled = compile_policy(&policy_path).expect("compiled policy");
    assert_eq!(compiled.rules.len(), 5);
    assert_eq!(
        render_json(&compiled).expect("json"),
        render_json(&compiled).expect("json")
    );

    let output = Command::new(env!("CARGO_BIN_EXE_cargo-policy"))
        .args([
            "compile",
            "--policy",
            policy_path.to_str().expect("utf8 policy path"),
            "--format",
            "json",
        ])
        .output()
        .expect("compile command");
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["tool"], "cargo-policy");
}

#[test]
fn cli_check_emits_a_violation_without_hiding_details() {
    let policy_path = write_policy("allow_registry = [\"not-the-registry\"]\n");
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-policy"))
        .args([
            "check",
            "--policy",
            policy_path.to_str().expect("utf8 policy path"),
            "--manifest-path",
            manifest_path.to_str().expect("utf8 manifest path"),
        ])
        .output()
        .expect("check command");

    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("registry-not-allowed"));
    assert!(text.contains("cargo-policy schema=1"));
}

fn write_policy(contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "cargo-policy-test-{}-{}.toml",
        std::process::id(),
        contents.len()
    ));
    fs::write(&path, contents).expect("write policy fixture");
    assert_eq!(
        fs::read_to_string(&path).expect("read policy fixture"),
        contents
    );
    path
}

#[allow(dead_code)]
fn default_policy() -> Policy {
    Policy::default()
}
