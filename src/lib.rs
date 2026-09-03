use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub allow_registry: Vec<String>,
    pub allow_git: Vec<String>,
    pub allow_path: Vec<String>,
    pub require_checksum: bool,
    pub forbid_unpinned_git: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SourceReplacement {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: String,
    pub code: String,
    pub package: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PackageReport {
    pub name: String,
    pub version: String,
    pub source_kind: String,
    pub source: String,
    pub source_ref: String,
    pub checksum: Option<String>,
    pub git_pinned: Option<bool>,
    pub path: Option<String>,
    pub workspace_member: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CheckReport {
    pub schema_version: u8,
    pub tool: String,
    pub status: String,
    pub workspace: String,
    pub policy: Policy,
    pub source_replacements: Vec<SourceReplacement>,
    pub packages: Vec<PackageReport>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckReport {
    pub fn exit_code(&self) -> i32 {
        status_exit_code(&self.status)
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ExplainReport {
    pub schema_version: u8,
    pub tool: String,
    pub status: String,
    pub package_query: String,
    pub matches: Vec<PackageReport>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ExplainReport {
    pub fn exit_code(&self) -> i32 {
        status_exit_code(&self.status)
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CompiledRule {
    pub code: String,
    pub behavior: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CompiledPolicy {
    pub schema_version: u8,
    pub tool: String,
    pub policy: Policy,
    pub rules: Vec<CompiledRule>,
}

#[derive(Debug)]
pub enum PolicyError {
    Io(String),
    InvalidPolicy(String),
    Cargo(String),
    Metadata(String),
    Lockfile(String),
    Configuration(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(detail) => write!(formatter, "I/O error: {detail}"),
            Self::InvalidPolicy(detail) => write!(formatter, "invalid policy: {detail}"),
            Self::Cargo(detail) => write!(formatter, "cargo metadata failed: {detail}"),
            Self::Metadata(detail) => write!(formatter, "invalid cargo metadata: {detail}"),
            Self::Lockfile(detail) => write!(formatter, "invalid Cargo.lock evidence: {detail}"),
            Self::Configuration(detail) => {
                write!(formatter, "invalid Cargo configuration: {detail}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

#[derive(Clone, Debug, Deserialize)]
struct MetadataDocument {
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
    workspace_root: String,
}

#[derive(Clone, Debug, Deserialize)]
struct MetadataPackage {
    name: String,
    version: String,
    id: String,
    source: Option<String>,
    manifest_path: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Lockfile {
    #[serde(default)]
    package: Vec<LockPackage>,
}

#[derive(Clone, Debug, Deserialize)]
struct LockPackage {
    name: String,
    version: String,
    source: Option<String>,
    checksum: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct LockIndex {
    packages: BTreeMap<(String, String, Option<String>), Option<String>>,
}

#[derive(Clone, Debug, Default)]
struct ConfigInfo {
    replacements: BTreeMap<String, String>,
    registry_names: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct SourceInfo {
    kind: String,
    source: String,
    source_ref: String,
    checksum: Option<String>,
    git_pinned: Option<bool>,
    path: Option<String>,
}

pub fn load_policy(path: &Path) -> Result<Policy, PolicyError> {
    let contents = fs::read_to_string(path).map_err(|error| PolicyError::Io(error.to_string()))?;
    let policy: Policy =
        toml::from_str(&contents).map_err(|error| PolicyError::InvalidPolicy(error.to_string()))?;
    validate_policy(&policy)?;
    Ok(policy)
}

pub fn compile_policy(path: &Path) -> Result<CompiledPolicy, PolicyError> {
    let policy = load_policy(path)?;
    Ok(CompiledPolicy {
        schema_version: SCHEMA_VERSION,
        tool: "cargo-policy".to_owned(),
        rules: compiled_rules(&policy),
        policy,
    })
}

pub fn check(policy_path: &Path, manifest_path: Option<&Path>) -> Result<CheckReport, PolicyError> {
    let policy = load_policy(policy_path)?;
    let metadata = run_cargo_metadata(manifest_path)?;
    let workspace_root = PathBuf::from(&metadata.workspace_root);
    let config = load_cargo_config(&workspace_root)?;
    let lock = load_lockfile(&workspace_root, policy.require_checksum)?;
    let git_pin_intents = collect_git_pin_intents(&metadata);
    Ok(evaluate(
        &policy,
        &metadata,
        &config,
        lock.as_ref(),
        &git_pin_intents,
    ))
}

pub fn explain(
    policy_path: &Path,
    package_query: &str,
    manifest_path: Option<&Path>,
) -> Result<ExplainReport, PolicyError> {
    let report = check(policy_path, manifest_path)?;
    let matches = report
        .packages
        .iter()
        .filter(|package| package.name == package_query)
        .cloned()
        .collect::<Vec<_>>();
    let mut diagnostics = report
        .diagnostics
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.package.is_none() || diagnostic.package.as_deref() == Some(package_query)
        })
        .collect::<Vec<_>>();
    let status = if matches.is_empty() {
        diagnostics.push(Diagnostic {
            severity: "unresolved".to_owned(),
            code: "package-not-found".to_owned(),
            package: Some(package_query.to_owned()),
            detail: "package name was not present in cargo metadata".to_owned(),
        });
        "unresolved".to_owned()
    } else if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "unresolved")
    {
        "unresolved".to_owned()
    } else if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error")
    {
        "violation".to_owned()
    } else {
        "ok".to_owned()
    };
    Ok(ExplainReport {
        schema_version: SCHEMA_VERSION,
        tool: "cargo-policy".to_owned(),
        status,
        package_query: package_query.to_owned(),
        matches,
        diagnostics,
    })
}

pub fn render_json<T: Serialize>(value: &T) -> serde_json::Result<String> {
    serde_json::to_string_pretty(value)
}

pub fn render_check_text(report: &CheckReport) -> String {
    let mut output = String::new();
    output.push_str("cargo-policy schema=1\n");
    output.push_str(&format!("status: {}\n", report.status));
    output.push_str(&format!("workspace: {}\n", report.workspace));
    output.push_str(&format!(
        "policy: allow_registry={} allow_git={} allow_path={} require_checksum={} forbid_unpinned_git={}\n",
        list_value(&report.policy.allow_registry),
        list_value(&report.policy.allow_git),
        list_value(&report.policy.allow_path),
        report.policy.require_checksum,
        report.policy.forbid_unpinned_git
    ));
    if report.source_replacements.is_empty() {
        output.push_str("source_replacements: none\n");
    } else {
        output.push_str("source_replacements:\n");
        for replacement in &report.source_replacements {
            output.push_str(&format!("- {} -> {}\n", replacement.from, replacement.to));
        }
    }
    output.push_str(&format!("packages: {}\n", report.packages.len()));
    for package in &report.packages {
        output.push_str(&format!(
            "- {} {} kind={} source={} ref={} checksum={} git_pinned={} path={} workspace_member={}\n",
            package.name,
            package.version,
            package.source_kind,
            package.source,
            package.source_ref,
            optional_value(package.checksum.as_deref()),
            package
                .git_pinned
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            optional_value(package.path.as_deref()),
            package.workspace_member
        ));
    }
    append_diagnostics(&mut output, &report.diagnostics);
    output
}

pub fn render_explain_text(report: &ExplainReport) -> String {
    let mut output = String::new();
    output.push_str("cargo-policy schema=1\n");
    output.push_str(&format!("status: {}\n", report.status));
    output.push_str(&format!("package_query: {}\n", report.package_query));
    if report.matches.is_empty() {
        output.push_str("matches: none\n");
    } else {
        output.push_str(&format!("matches: {}\n", report.matches.len()));
        for package in &report.matches {
            output.push_str(&format!(
                "- {} {} kind={} source={} ref={} checksum={} git_pinned={} path={} workspace_member={}\n",
                package.name,
                package.version,
                package.source_kind,
                package.source,
                package.source_ref,
                optional_value(package.checksum.as_deref()),
                package
                    .git_pinned
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
                optional_value(package.path.as_deref()),
                package.workspace_member
            ));
        }
    }
    append_diagnostics(&mut output, &report.diagnostics);
    output
}

pub fn render_compiled_text(policy: &CompiledPolicy) -> String {
    let mut output = String::new();
    output.push_str("cargo-policy schema=1\n");
    output.push_str("compiled: true\n");
    output.push_str(&format!(
        "policy: allow_registry={} allow_git={} allow_path={} require_checksum={} forbid_unpinned_git={}\n",
        list_value(&policy.policy.allow_registry),
        list_value(&policy.policy.allow_git),
        list_value(&policy.policy.allow_path),
        policy.policy.require_checksum,
        policy.policy.forbid_unpinned_git
    ));
    output.push_str("rules:\n");
    for rule in &policy.rules {
        output.push_str(&format!("- {}: {}\n", rule.code, rule.behavior));
    }
    output
}

fn validate_policy(policy: &Policy) -> Result<(), PolicyError> {
    for pattern in policy
        .allow_registry
        .iter()
        .chain(policy.allow_git.iter())
        .chain(policy.allow_path.iter())
    {
        if pattern.is_empty() || pattern.chars().any(char::is_control) {
            return Err(PolicyError::InvalidPolicy(
                "allowlist patterns must be nonempty and free of control characters".to_owned(),
            ));
        }
    }
    Ok(())
}

fn compiled_rules(policy: &Policy) -> Vec<CompiledRule> {
    let mut rules = vec![
        CompiledRule {
            code: "registry-allowlist".to_owned(),
            behavior: if policy.allow_registry.is_empty() {
                "deny registry sources".to_owned()
            } else {
                format!(
                    "allow registry labels matching {}",
                    list_value(&policy.allow_registry)
                )
            },
        },
        CompiledRule {
            code: "git-allowlist".to_owned(),
            behavior: if policy.allow_git.is_empty() {
                "deny git sources".to_owned()
            } else {
                format!(
                    "allow git references matching {}",
                    list_value(&policy.allow_git)
                )
            },
        },
        CompiledRule {
            code: "path-allowlist".to_owned(),
            behavior: if policy.allow_path.is_empty() {
                "deny external path sources".to_owned()
            } else {
                format!("allow paths matching {}", list_value(&policy.allow_path))
            },
        },
    ];
    rules.push(CompiledRule {
        code: "checksum".to_owned(),
        behavior: if policy.require_checksum {
            "require a checksum for every registry package".to_owned()
        } else {
            "do not require registry checksums".to_owned()
        },
    });
    rules.push(CompiledRule {
        code: "git-revision".to_owned(),
        behavior: if policy.forbid_unpinned_git {
            "require an explicit immutable git revision".to_owned()
        } else {
            "allow unpinned git revisions".to_owned()
        },
    });
    rules
}

fn run_cargo_metadata(manifest_path: Option<&Path>) -> Result<MetadataDocument, PolicyError> {
    let mut command = Command::new("cargo");
    command.args(["metadata", "--format-version", "1", "--locked"]);
    if let Some(path) = manifest_path {
        command.args(["--manifest-path", path.to_string_lossy().as_ref()]);
    }
    let output = command
        .output()
        .map_err(|error| PolicyError::Cargo(error.to_string()))?;
    if !output.status.success() {
        return Err(PolicyError::Cargo(clean_process_output(&output.stderr)));
    }
    serde_json::from_slice(&output.stdout).map_err(|error| PolicyError::Metadata(error.to_string()))
}

fn load_lockfile(workspace_root: &Path, required: bool) -> Result<Option<LockIndex>, PolicyError> {
    let path = workspace_root.join("Cargo.lock");
    if !path.exists() {
        if required {
            return Err(PolicyError::Lockfile(format!(
                "{} is required by require_checksum",
                path.display()
            )));
        }
        return Ok(None);
    }
    let contents = fs::read_to_string(&path).map_err(|error| PolicyError::Io(error.to_string()))?;
    let lockfile: Lockfile =
        toml::from_str(&contents).map_err(|error| PolicyError::Lockfile(error.to_string()))?;
    let mut index = LockIndex::default();
    for package in lockfile.package {
        let key = (package.name, package.version, package.source);
        if index
            .packages
            .insert(key.clone(), package.checksum)
            .is_some()
        {
            return Err(PolicyError::Lockfile(format!(
                "duplicate package entry for {} {}",
                key.0, key.1
            )));
        }
    }
    Ok(Some(index))
}

fn load_cargo_config(workspace_root: &Path) -> Result<ConfigInfo, PolicyError> {
    let cargo_dir = workspace_root.join(".cargo");
    let path = [cargo_dir.join("config.toml"), cargo_dir.join("config")]
        .into_iter()
        .find(|candidate| candidate.exists());
    let Some(path) = path else {
        return Ok(ConfigInfo::default());
    };
    let contents = fs::read_to_string(&path).map_err(|error| PolicyError::Io(error.to_string()))?;
    let value: toml::Value =
        toml::from_str(&contents).map_err(|error| PolicyError::Configuration(error.to_string()))?;
    parse_cargo_config(&value)
}

fn parse_cargo_config(value: &toml::Value) -> Result<ConfigInfo, PolicyError> {
    let mut info = ConfigInfo::default();
    if let Some(source_table) = value.get("source") {
        let Some(source_table) = source_table.as_table() else {
            return Err(PolicyError::Configuration(
                "[source] must be a table".to_owned(),
            ));
        };
        for (name, entry) in source_table {
            let Some(entry) = entry.as_table() else {
                return Err(PolicyError::Configuration(format!(
                    "source.{name} must be a table"
                )));
            };
            if let Some(replace_with) = entry.get("replace-with") {
                let Some(replace_with) = replace_with.as_str() else {
                    return Err(PolicyError::Configuration(format!(
                        "source.{name}.replace-with must be a string"
                    )));
                };
                info.replacements
                    .insert(name.to_owned(), replace_with.to_owned());
            }
        }
    }
    if let Some(registries) = value.get("registries") {
        let Some(registries) = registries.as_table() else {
            return Err(PolicyError::Configuration(
                "[registries] must be a table".to_owned(),
            ));
        };
        for (name, entry) in registries {
            let Some(entry) = entry.as_table() else {
                return Err(PolicyError::Configuration(format!(
                    "registries.{name} must be a table"
                )));
            };
            if let Some(index) = entry.get("index") {
                let Some(index) = index.as_str() else {
                    return Err(PolicyError::Configuration(format!(
                        "registries.{name}.index must be a string"
                    )));
                };
                info.registry_names
                    .insert(normalize_registry_endpoint(index), name.to_owned());
            }
        }
    }
    validate_replacements(&info.replacements)?;
    Ok(info)
}

fn validate_replacements(replacements: &BTreeMap<String, String>) -> Result<(), PolicyError> {
    for source in replacements.keys() {
        let mut seen = BTreeSet::new();
        let mut current = source.as_str();
        while let Some(next) = replacements.get(current) {
            if !seen.insert(current.to_owned()) {
                return Err(PolicyError::Configuration(format!(
                    "source replacement cycle includes {source}"
                )));
            }
            current = next;
        }
    }
    Ok(())
}

fn collect_git_pin_intents(metadata: &MetadataDocument) -> BTreeMap<String, bool> {
    let mut intents = BTreeMap::new();
    for package in &metadata.packages {
        let Ok(contents) = fs::read_to_string(&package.manifest_path) else {
            continue;
        };
        let Ok(value) = contents.parse::<toml::Value>() else {
            continue;
        };
        collect_git_pin_intents_from_table(&value, &mut intents);
    }
    intents
}

fn collect_git_pin_intents_from_table(value: &toml::Value, intents: &mut BTreeMap<String, bool>) {
    let Some(table) = value.as_table() else {
        return;
    };
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(dependencies) = table.get(section).and_then(toml::Value::as_table) {
            collect_git_pin_intents_from_dependency_table(dependencies, intents);
        }
    }
    if let Some(targets) = table.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            collect_git_pin_intents_from_table(target, intents);
        }
    }
}

fn collect_git_pin_intents_from_dependency_table(
    dependencies: &toml::map::Map<String, toml::Value>,
    intents: &mut BTreeMap<String, bool>,
) {
    for dependency in dependencies.values() {
        let Some(dependency) = dependency.as_table() else {
            continue;
        };
        let Some(git) = dependency.get("git").and_then(toml::Value::as_str) else {
            continue;
        };
        let pinned = dependency
            .get("rev")
            .and_then(toml::Value::as_str)
            .is_some_and(|revision| !revision.is_empty());
        intents
            .entry(normalize_git_reference(git))
            .and_modify(|current| *current &= pinned)
            .or_insert(pinned);
    }
}

fn evaluate(
    policy: &Policy,
    metadata: &MetadataDocument,
    config: &ConfigInfo,
    lock: Option<&LockIndex>,
    git_pin_intents: &BTreeMap<String, bool>,
) -> CheckReport {
    let workspace_members = metadata.workspace_members.iter().collect::<BTreeSet<_>>();
    let replacements = config
        .replacements
        .iter()
        .map(|(from, to)| SourceReplacement {
            from: from.clone(),
            to: to.clone(),
        })
        .collect::<Vec<_>>();
    let mut packages = Vec::new();
    let mut diagnostics = Vec::new();

    for replacement in &replacements {
        diagnostics.push(Diagnostic {
            severity: "info".to_owned(),
            code: "source-replacement".to_owned(),
            package: None,
            detail: format!(
                "Cargo source {} is replaced with {}",
                replacement.from, replacement.to
            ),
        });
    }

    for package in &metadata.packages {
        let workspace_member = workspace_members.contains(&package.id);
        let source = source_info(
            package,
            workspace_member,
            Path::new(&metadata.workspace_root),
            config,
            lock,
            git_pin_intents,
        );
        let source = match source {
            Ok(source) => source,
            Err(detail) => {
                diagnostics.push(Diagnostic {
                    severity: "unresolved".to_owned(),
                    code: "source-unresolved".to_owned(),
                    package: Some(package.name.clone()),
                    detail,
                });
                SourceInfo {
                    kind: "unknown".to_owned(),
                    source: "unknown".to_owned(),
                    source_ref: "unknown".to_owned(),
                    checksum: None,
                    git_pinned: None,
                    path: None,
                }
            }
        };
        let record = PackageReport {
            name: package.name.clone(),
            version: package.version.clone(),
            source_kind: source.kind.clone(),
            source: source.source.clone(),
            source_ref: source.source_ref.clone(),
            checksum: source.checksum.clone(),
            git_pinned: source.git_pinned,
            path: source.path.clone(),
            workspace_member,
        };
        if !workspace_member {
            apply_source_policy(policy, package, &source, &mut diagnostics);
        }
        packages.push(record);
    }

    packages.sort_by(|left, right| {
        (
            &left.name,
            &left.version,
            &left.source_kind,
            &left.source_ref,
        )
            .cmp(&(
                &right.name,
                &right.version,
                &right.source_kind,
                &right.source_ref,
            ))
    });
    add_multiple_version_diagnostics(&packages, &mut diagnostics);
    diagnostics.sort_by(|left, right| {
        (&left.severity, &left.code, &left.package, &left.detail).cmp(&(
            &right.severity,
            &right.code,
            &right.package,
            &right.detail,
        ))
    });
    let status = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "unresolved")
    {
        "unresolved"
    } else if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error")
    {
        "violation"
    } else {
        "ok"
    };
    CheckReport {
        schema_version: SCHEMA_VERSION,
        tool: "cargo-policy".to_owned(),
        status: status.to_owned(),
        workspace: redact_path(&metadata.workspace_root),
        policy: policy.clone(),
        source_replacements: replacements,
        packages,
        diagnostics,
    }
}

fn source_info(
    package: &MetadataPackage,
    workspace_member: bool,
    workspace_root: &Path,
    config: &ConfigInfo,
    lock: Option<&LockIndex>,
    git_pin_intents: &BTreeMap<String, bool>,
) -> Result<SourceInfo, String> {
    if workspace_member {
        return Ok(SourceInfo {
            kind: "workspace".to_owned(),
            source: "workspace".to_owned(),
            source_ref: "workspace".to_owned(),
            checksum: None,
            git_pinned: None,
            path: Some(display_path(
                Path::new(&package.manifest_path),
                workspace_root,
            )),
        });
    }
    let Some(source) = package.source.as_deref() else {
        return Ok(SourceInfo {
            kind: "path".to_owned(),
            source: "path".to_owned(),
            source_ref: "path".to_owned(),
            checksum: None,
            git_pinned: None,
            path: Some(display_path(
                Path::new(&package.manifest_path),
                workspace_root,
            )),
        });
    };
    if source.starts_with("registry+") {
        let endpoint = normalize_registry_endpoint(source);
        let label = if is_crates_io_endpoint(&endpoint) {
            effective_source_name("crates-io", config)
        } else {
            config
                .registry_names
                .get(&endpoint)
                .cloned()
                .unwrap_or_else(|| endpoint_display(&endpoint))
        };
        let checksum = lock.and_then(|index| {
            index
                .packages
                .get(&(
                    package.name.clone(),
                    package.version.clone(),
                    Some(source.to_owned()),
                ))
                .cloned()
                .flatten()
        });
        return Ok(SourceInfo {
            kind: "registry".to_owned(),
            source: label,
            source_ref: endpoint_display(&endpoint),
            checksum,
            git_pinned: None,
            path: None,
        });
    }
    if source.starts_with("git+") {
        let reference = normalize_git_reference(source);
        let locked_revision = git_lock_revision(source);
        let pinned = git_pin_intents
            .get(&reference)
            .copied()
            .unwrap_or(locked_revision);
        return Ok(SourceInfo {
            kind: "git".to_owned(),
            source: "git".to_owned(),
            source_ref: reference,
            checksum: None,
            git_pinned: Some(pinned),
            path: None,
        });
    }
    Err(format!("unsupported Cargo source `{source}`"))
}

fn apply_source_policy(
    policy: &Policy,
    package: &MetadataPackage,
    source: &SourceInfo,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match source.kind.as_str() {
        "registry" => {
            if !matches_any(&policy.allow_registry, &source.source) {
                diagnostics.push(Diagnostic {
                    severity: "error".to_owned(),
                    code: "registry-not-allowed".to_owned(),
                    package: Some(package.name.clone()),
                    detail: format!("registry `{}` is not in allow_registry", source.source),
                });
            }
            if policy.require_checksum && source.checksum.is_none() {
                diagnostics.push(Diagnostic {
                    severity: "error".to_owned(),
                    code: "missing-checksum".to_owned(),
                    package: Some(package.name.clone()),
                    detail: "registry package has no checksum in Cargo.lock".to_owned(),
                });
            }
        }
        "git" => {
            if !matches_any(&policy.allow_git, &source.source_ref) {
                diagnostics.push(Diagnostic {
                    severity: "error".to_owned(),
                    code: "git-not-allowed".to_owned(),
                    package: Some(package.name.clone()),
                    detail: format!("git reference `{}` is not in allow_git", source.source_ref),
                });
            }
            if policy.forbid_unpinned_git && source.git_pinned != Some(true) {
                diagnostics.push(Diagnostic {
                    severity: "error".to_owned(),
                    code: "unpinned-git-revision".to_owned(),
                    package: Some(package.name.clone()),
                    detail: "git dependency is not declared with an explicit rev".to_owned(),
                });
            }
        }
        "path" => {
            let path = source.path.as_deref().unwrap_or("path");
            if !matches_any(&policy.allow_path, path) {
                diagnostics.push(Diagnostic {
                    severity: "error".to_owned(),
                    code: "path-not-allowed".to_owned(),
                    package: Some(package.name.clone()),
                    detail: format!("path `{path}` is not in allow_path"),
                });
            }
        }
        "workspace" => {}
        _ => diagnostics.push(Diagnostic {
            severity: "unresolved".to_owned(),
            code: "unsupported-source".to_owned(),
            package: Some(package.name.clone()),
            detail: "package source could not be classified safely".to_owned(),
        }),
    }
}

fn add_multiple_version_diagnostics(packages: &[PackageReport], diagnostics: &mut Vec<Diagnostic>) {
    let mut versions = BTreeMap::<String, BTreeSet<String>>::new();
    for package in packages.iter().filter(|package| !package.workspace_member) {
        versions
            .entry(package.name.clone())
            .or_default()
            .insert(package.version.clone());
    }
    for (name, versions) in versions {
        if versions.len() > 1 {
            diagnostics.push(Diagnostic {
                severity: "info".to_owned(),
                code: "multiple-versions".to_owned(),
                package: Some(name),
                detail: format!(
                    "resolved package has versions {}",
                    versions.into_iter().collect::<Vec<_>>().join(", ")
                ),
            });
        }
    }
}

fn effective_source_name(source: &str, config: &ConfigInfo) -> String {
    let mut current = source.to_owned();
    let mut seen = BTreeSet::new();
    while let Some(next) = config.replacements.get(&current) {
        if !seen.insert(current.clone()) {
            break;
        }
        current = next.clone();
    }
    if current == "crates-io" {
        "crates.io".to_owned()
    } else {
        current
    }
}

#[cfg(test)]
fn load_lockfile_for_tests(contents: &str) -> LockIndex {
    let lockfile: Lockfile = toml::from_str(contents).expect("test lockfile");
    LockIndex {
        packages: lockfile
            .package
            .into_iter()
            .map(|package| {
                (
                    (package.name, package.version, package.source),
                    package.checksum,
                )
            })
            .collect(),
    }
}

fn normalize_registry_endpoint(source: &str) -> String {
    source
        .strip_prefix("registry+")
        .unwrap_or(source)
        .strip_prefix("sparse+")
        .unwrap_or_else(|| source.strip_prefix("registry+").unwrap_or(source))
        .trim_end_matches('/')
        .to_owned()
}

fn is_crates_io_endpoint(endpoint: &str) -> bool {
    endpoint == "https://github.com/rust-lang/crates.io-index"
}

fn endpoint_display(endpoint: &str) -> String {
    endpoint
        .strip_prefix("https://")
        .or_else(|| endpoint.strip_prefix("http://"))
        .or_else(|| endpoint.strip_prefix("ssh://"))
        .unwrap_or(endpoint)
        .trim_start_matches("git@")
        .to_owned()
}

fn normalize_git_reference(source: &str) -> String {
    let source = source.strip_prefix("git+").unwrap_or(source);
    let source = source.split(['?', '#']).next().unwrap_or(source);
    let source = source
        .strip_prefix("https://")
        .or_else(|| source.strip_prefix("http://"))
        .or_else(|| source.strip_prefix("ssh://"))
        .unwrap_or(source);
    let source = source
        .rsplit_once('@')
        .map(|(_, value)| value)
        .unwrap_or(source);
    source
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_owned()
}

fn git_lock_revision(source: &str) -> bool {
    let Some((_, fragment)) = source.rsplit_once('#') else {
        return false;
    };
    let fragment = fragment.trim();
    (7..=64).contains(&fragment.len()) && fragment.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn display_path(path: &Path, workspace_root: &Path) -> String {
    if let Ok(relative) = path.parent().unwrap_or(path).strip_prefix(workspace_root) {
        let value = relative.to_string_lossy();
        if value.is_empty() {
            "workspace".to_owned()
        } else {
            value.replace('\\', "/")
        }
    } else {
        let name = path
            .parent()
            .and_then(Path::file_name)
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "path".to_owned());
        format!("<external>/{name}")
    }
}

fn redact_path(value: &str) -> String {
    if value.starts_with('/') {
        let name = Path::new(value)
            .file_name()
            .map(|item| item.to_string_lossy().into_owned())
            .unwrap_or_else(|| "path".to_owned());
        format!("<absolute>/{name}")
    } else {
        value.to_owned()
    }
}

fn matches_any(patterns: &[String], value: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| wildcard_match(pattern, value))
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let mut pattern_index = 0;
    let mut value_index = 0;
    let mut star_index = None;
    let mut star_value_index = 0;
    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == value[value_index] || pattern[pattern_index] == b'?')
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            star_value_index = value_index;
            pattern_index += 1;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }
    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn status_exit_code(status: &str) -> i32 {
    match status {
        "ok" => 0,
        "violation" => 2,
        _ => 3,
    }
}

fn list_value(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

fn optional_value(value: Option<&str>) -> String {
    value.unwrap_or("none").to_owned()
}

fn append_diagnostics(output: &mut String, diagnostics: &[Diagnostic]) {
    if diagnostics.is_empty() {
        output.push_str("diagnostics: none\n");
    } else {
        output.push_str("diagnostics:\n");
        for diagnostic in diagnostics {
            output.push_str(&format!(
                "- severity={} code={} package={} detail={}\n",
                diagnostic.severity,
                diagnostic.code,
                optional_value(diagnostic.package.as_deref()),
                diagnostic.detail
            ));
        }
    }
}

fn clean_process_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|character| !character.is_control() || *character == '\n')
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(
        name: &str,
        version: &str,
        id: &str,
        source: Option<&str>,
        manifest_path: &str,
    ) -> MetadataPackage {
        MetadataPackage {
            name: name.to_owned(),
            version: version.to_owned(),
            id: id.to_owned(),
            source: source.map(str::to_owned),
            manifest_path: manifest_path.to_owned(),
        }
    }

    fn metadata(packages: Vec<MetadataPackage>) -> MetadataDocument {
        MetadataDocument {
            packages,
            workspace_members: vec!["path+file:///workspace#app@1.0.0".to_owned()],
            workspace_root: "/workspace".to_owned(),
        }
    }

    fn registry_source() -> &'static str {
        "registry+https://github.com/rust-lang/crates.io-index"
    }

    #[test]
    fn registry_allowlist_and_checksum_pass() {
        let policy = Policy {
            allow_registry: vec!["crates.io".to_owned()],
            require_checksum: true,
            ..Policy::default()
        };
        let metadata = metadata(vec![
            package(
                "app",
                "1.0.0",
                "path+file:///workspace#app@1.0.0",
                None,
                "/workspace/Cargo.toml",
            ),
            package(
                "dep",
                "1.0.0",
                "registry+https://github.com/rust-lang/crates.io-index#dep@1.0.0",
                Some(registry_source()),
                "/cache/dep/Cargo.toml",
            ),
        ]);
        let lock = load_lockfile_for_tests(&format!(
            "[[package]]\nname = \"dep\"\nversion = \"1.0.0\"\nsource = \"{}\"\nchecksum = \"abc\"\n",
            registry_source()
        ));
        let report = evaluate(
            &policy,
            &metadata,
            &ConfigInfo::default(),
            Some(&lock),
            &BTreeMap::new(),
        );
        assert_eq!(report.status, "ok");
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn git_allowlist_reports_unpinned_revision() {
        let policy = Policy {
            allow_git: vec!["github.com/org/*".to_owned()],
            forbid_unpinned_git: true,
            ..Policy::default()
        };
        let source = "git+https://github.com/org/repo";
        let metadata = metadata(vec![package(
            "git-dep",
            "1.0.0",
            "git+https://github.com/org/repo#git-dep@1.0.0",
            Some(source),
            "/cache/git-dep/Cargo.toml",
        )]);
        let mut intents = BTreeMap::new();
        intents.insert(normalize_git_reference(source), false);
        let report = evaluate(&policy, &metadata, &ConfigInfo::default(), None, &intents);
        assert_eq!(report.status, "violation");
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "unpinned-git-revision")
        );
    }

    #[test]
    fn path_allowlist_is_checked() {
        let policy = Policy {
            allow_path: vec!["vendor/*".to_owned()],
            ..Policy::default()
        };
        let metadata = metadata(vec![package(
            "path-dep",
            "1.0.0",
            "path+file:///workspace/vendor/path-dep#path-dep@1.0.0",
            None,
            "/workspace/vendor/path-dep/Cargo.toml",
        )]);
        let report = evaluate(
            &policy,
            &metadata,
            &ConfigInfo::default(),
            None,
            &BTreeMap::new(),
        );
        assert_eq!(report.status, "ok");
        assert_eq!(report.packages[0].path.as_deref(), Some("vendor/path-dep"));
    }

    #[test]
    fn multiple_versions_are_deterministic_information() {
        let policy = Policy {
            allow_registry: vec!["crates.io".to_owned()],
            ..Policy::default()
        };
        let metadata = metadata(vec![
            package(
                "dep",
                "1.0.0",
                "registry+https://github.com/rust-lang/crates.io-index#dep@1.0.0",
                Some(registry_source()),
                "/cache/dep1/Cargo.toml",
            ),
            package(
                "dep",
                "2.0.0",
                "registry+https://github.com/rust-lang/crates.io-index#dep@2.0.0",
                Some(registry_source()),
                "/cache/dep2/Cargo.toml",
            ),
        ]);
        let first = evaluate(
            &policy,
            &metadata,
            &ConfigInfo::default(),
            None,
            &BTreeMap::new(),
        );
        let second = evaluate(
            &policy,
            &metadata,
            &ConfigInfo::default(),
            None,
            &BTreeMap::new(),
        );
        assert_eq!(first.status, "ok");
        assert!(
            first
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "multiple-versions")
        );
        assert_eq!(
            render_json(&first).expect("first json"),
            render_json(&second).expect("second json")
        );
    }

    #[test]
    fn missing_checksum_is_a_violation() {
        let policy = Policy {
            allow_registry: vec!["crates.io".to_owned()],
            require_checksum: true,
            ..Policy::default()
        };
        let metadata = metadata(vec![package(
            "dep",
            "1.0.0",
            "registry+https://github.com/rust-lang/crates.io-index#dep@1.0.0",
            Some(registry_source()),
            "/cache/dep/Cargo.toml",
        )]);
        let lock = load_lockfile_for_tests(&format!(
            "[[package]]\nname = \"dep\"\nversion = \"1.0.0\"\nsource = \"{}\"\n",
            registry_source()
        ));
        let report = evaluate(
            &policy,
            &metadata,
            &ConfigInfo::default(),
            Some(&lock),
            &BTreeMap::new(),
        );
        assert_eq!(report.status, "violation");
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing-checksum")
        );
    }

    #[test]
    fn source_replacement_changes_effective_registry_label() {
        let policy = Policy {
            allow_registry: vec!["corp".to_owned()],
            ..Policy::default()
        };
        let metadata = metadata(vec![package(
            "dep",
            "1.0.0",
            "registry+https://github.com/rust-lang/crates.io-index#dep@1.0.0",
            Some(registry_source()),
            "/cache/dep/Cargo.toml",
        )]);
        let mut config = ConfigInfo::default();
        config
            .replacements
            .insert("crates-io".to_owned(), "corp".to_owned());
        let report = evaluate(&policy, &metadata, &config, None, &BTreeMap::new());
        assert_eq!(report.status, "ok");
        assert_eq!(report.packages[0].source, "corp");
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "source-replacement")
        );
    }

    #[test]
    fn malformed_policy_is_rejected() {
        let result = toml::from_str::<Policy>("unknown = true");
        assert!(result.is_err());
    }
}
