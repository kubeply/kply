//! Repository automation placeholder for Kply development tasks.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_norway::Value as YamlValue;

static YAML_JOBS_KEY: LazyLock<YamlValue> = LazyLock::new(|| YamlValue::String("jobs".to_owned()));
static YAML_BRANCHES_KEY: LazyLock<YamlValue> =
    LazyLock::new(|| YamlValue::String("branches".to_owned()));
static YAML_CONTENTS_KEY: LazyLock<YamlValue> =
    LazyLock::new(|| YamlValue::String("contents".to_owned()));
static YAML_MERGE_GROUP_KEY: LazyLock<YamlValue> =
    LazyLock::new(|| YamlValue::String("merge_group".to_owned()));
static YAML_PERMISSIONS_KEY: LazyLock<YamlValue> =
    LazyLock::new(|| YamlValue::String("permissions".to_owned()));
static YAML_ON_KEY: LazyLock<YamlValue> = LazyLock::new(|| YamlValue::String("on".to_owned()));
static YAML_PULL_REQUEST_KEY: LazyLock<YamlValue> =
    LazyLock::new(|| YamlValue::String("pull_request".to_owned()));
static YAML_PUSH_KEY: LazyLock<YamlValue> = LazyLock::new(|| YamlValue::String("push".to_owned()));
static YAML_RUN_KEY: LazyLock<YamlValue> = LazyLock::new(|| YamlValue::String("run".to_owned()));
static YAML_STEPS_KEY: LazyLock<YamlValue> =
    LazyLock::new(|| YamlValue::String("steps".to_owned()));
static YAML_TAGS_KEY: LazyLock<YamlValue> = LazyLock::new(|| YamlValue::String("tags".to_owned()));
static YAML_USES_KEY: LazyLock<YamlValue> = LazyLock::new(|| YamlValue::String("uses".to_owned()));
static SECRET_FIELD_ACCESS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\.\s*(data|string_data)\b")
        .expect("Secret field access regex should compile")
});
static SECRET_TYPED_IDENTIFIER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?:&\s*(?:'[A-Za-z_][A-Za-z0-9_]*\s*)?(?:mut\s+)?)?Secret\b",
    )
        .expect("Secret typed identifier regex should compile")
});

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_owned());

    match command.as_str() {
        "help" => {
            println!("available tasks:");
            println!("  check-ci-workflow verify pull-request CI release gates");
            println!("  check-crate-inventory-docs verify docs list workspace crates");
            println!("  check-demo-docs verify local demo docs stay linked and bounded");
            println!("  check-deny-config verify cargo-deny policy strictness");
            println!("  check-fixture-directories verify fixture directory skeleton");
            println!("  check-fixture-naming-docs verify fixture naming docs");
            println!("  check-fixture-testing-docs verify fixture testing guidance");
            println!("  check-feedback-triage-docs verify adoption feedback triage docs");
            println!("  check-future-session-docs verify future session docs are explicit");
            println!("  check-issue-templates verify feedback issue templates");
            println!(
                "  check-known-limitations-docs verify first-release limitations stay explicit"
            );
            println!("  check-license-files verify Apache-2.0 license and notice files");
            println!("  check-module-docs  verify crate source files start with module docs");
            println!(
                "  check-no-secret-content-reads verify Kubernetes Secret contents stay unread"
            );
            println!("  check-placeholder-docs verify public docs describe placeholder status");
            println!("  check-placeholders verify product crates expose placeholder markers only");
            println!("  check-report-language verify reports do not overclaim deployment safety");
            println!("  check-readme-roadmap-link verify README links the roadmap");
            println!("  check-release-planning verify cargo-dist release packaging stays pinned");
            println!("  check-security-assumptions-docs verify first-release trust assumptions");
            println!("  check-security-policy verify private vulnerability reporting docs");
            println!("  check-toolchain-pin verify Rust toolchain pinning");
            println!("  help               print this message");
            println!("  validate           print the validation command list");
        }
        "check-ci-workflow" => {
            check_ci_workflow()?;
        }
        "check-crate-inventory-docs" => {
            check_crate_inventory_docs()?;
        }
        "check-demo-docs" => {
            check_demo_docs()?;
        }
        "check-deny-config" => {
            check_deny_config()?;
        }
        "check-fixture-directories" => {
            check_fixture_directories()?;
        }
        "check-fixture-naming-docs" => {
            check_fixture_naming_docs()?;
        }
        "check-fixture-testing-docs" => {
            check_fixture_testing_docs()?;
        }
        "check-feedback-triage-docs" => {
            check_feedback_triage_docs()?;
        }
        "check-future-session-docs" => {
            check_future_session_docs()?;
        }
        "check-issue-templates" => {
            check_issue_templates()?;
        }
        "check-known-limitations-docs" => {
            check_known_limitations_docs()?;
        }
        "check-license-files" => {
            check_license_files()?;
        }
        "check-module-docs" => {
            check_module_docs()?;
        }
        "check-no-secret-content-reads" => {
            check_no_secret_content_reads()?;
        }
        "check-placeholder-docs" => {
            check_placeholder_docs()?;
        }
        "check-placeholders" => {
            check_placeholders()?;
        }
        "check-report-language" => {
            check_report_language()?;
        }
        "check-readme-roadmap-link" => {
            check_readme_roadmap_link()?;
        }
        "check-release-planning" => {
            check_release_planning()?;
        }
        "check-security-assumptions-docs" => {
            check_security_assumptions_docs()?;
        }
        "check-security-policy" => {
            check_security_policy()?;
        }
        "check-toolchain-pin" => {
            check_toolchain_pin()?;
        }
        "validate" => {
            println!("cargo fmt --all -- --check");
            println!("cargo check --all-targets --all-features --locked");
            println!("cargo clippy --all-targets --all-features --locked -- -D warnings");
            println!("cargo test --all-targets --all-features --locked");
            println!("cargo test -p kply-test --locked");
            println!("cargo deny check");
            println!("cargo xtask check-ci-workflow");
            println!("cargo xtask check-crate-inventory-docs");
            println!("cargo xtask check-demo-docs");
            println!("cargo xtask check-deny-config");
            println!("cargo xtask check-fixture-directories");
            println!("cargo xtask check-fixture-naming-docs");
            println!("cargo xtask check-fixture-testing-docs");
            println!("cargo xtask check-feedback-triage-docs");
            println!("cargo xtask check-future-session-docs");
            println!("cargo xtask check-issue-templates");
            println!("cargo xtask check-known-limitations-docs");
            println!("cargo xtask check-license-files");
            println!("cargo xtask check-module-docs");
            println!("cargo xtask check-no-secret-content-reads");
            println!("cargo xtask check-placeholder-docs");
            println!("cargo xtask check-placeholders");
            println!("cargo xtask check-report-language");
            println!("cargo xtask check-readme-roadmap-link");
            println!("cargo xtask check-release-planning");
            println!("cargo xtask check-security-assumptions-docs");
            println!("cargo xtask check-security-policy");
            println!("cargo xtask check-toolchain-pin");
        }
        unknown => bail!("unknown xtask command: {unknown}"),
    }

    Ok(())
}

fn check_module_docs() -> Result<()> {
    let crate_sources = collect_crate_sources("crates")?;
    let mut missing_docs = Vec::new();

    for source_path in crate_sources {
        let source = std::fs::read_to_string(&source_path)?;
        let first_line = source.lines().next().unwrap_or_default();

        if !first_line.starts_with("//!")
            || source
                .lines()
                .nth(1)
                .is_some_and(|line| line.starts_with("//!"))
        {
            missing_docs.push(source_path);
        }
    }

    if !missing_docs.is_empty() {
        for source_path in &missing_docs {
            eprintln!("missing module docstring: {}", source_path.display());
        }
        bail!(
            "{} crate source file(s) missing module docs",
            missing_docs.len()
        );
    }

    Ok(())
}

fn check_ci_workflow() -> Result<()> {
    check_ci_workflow_inner(".github/workflows/ci.yml".as_ref())
}

fn check_no_secret_content_reads() -> Result<()> {
    let mut source_paths = Vec::new();
    for root in secret_content_guard_source_roots() {
        source_paths.extend(collect_crate_sources(root)?);
    }
    source_paths.sort();
    source_paths.dedup();

    check_no_secret_content_reads_inner(source_paths, forbidden_secret_content_patterns())
}

fn check_report_language() -> Result<()> {
    check_report_language_inner(
        report_language_paths(),
        forbidden_report_overclaim_phrases(),
    )
}

fn check_crate_inventory_docs() -> Result<()> {
    let doc_paths = ["AGENTS.md", "CONTRIBUTING.md", "crates/README.md"];

    check_crate_inventory_docs_inner("Cargo.toml".as_ref(), doc_paths, workspace_crates())
}

fn check_demo_docs() -> Result<()> {
    check_demo_docs_inner([
        PathBuf::from("README.md"),
        PathBuf::from("docs/demo-kind.md"),
        PathBuf::from("docs/demo-agent.md"),
        PathBuf::from("fixtures/demo/ecommerce-basic/README.md"),
    ])
}

fn check_deny_config() -> Result<()> {
    check_deny_config_inner("deny.toml".as_ref())
}

fn check_license_files() -> Result<()> {
    check_license_files_inner(
        "LICENSE".as_ref(),
        "NOTICE".as_ref(),
        "Cargo.toml".as_ref(),
        workspace_crates(),
    )
}

fn check_fixture_directories() -> Result<()> {
    check_fixture_directories_inner("fixtures".as_ref(), required_fixture_directories())
}

fn check_fixture_naming_docs() -> Result<()> {
    check_fixture_naming_docs_inner("fixtures/README.md".as_ref())
}

fn check_fixture_testing_docs() -> Result<()> {
    check_fixture_testing_docs_inner("fixtures/README.md".as_ref())
}

fn check_feedback_triage_docs() -> Result<()> {
    check_feedback_triage_docs_inner("docs/feedback-triage.md".into())
}

fn check_future_session_docs() -> Result<()> {
    check_future_session_docs_inner([
        "README.md".into(),
        "docs/architecture.md".into(),
        "docs/product.md".into(),
    ])
}

fn check_issue_templates() -> Result<()> {
    check_issue_templates_inner([
        ".github/ISSUE_TEMPLATE/config.yml".into(),
        ".github/ISSUE_TEMPLATE/routing-environment.yml".into(),
        ".github/ISSUE_TEMPLATE/kubernetes-discovery-bug.yml".into(),
        ".github/ISSUE_TEMPLATE/session-planning-gap.yml".into(),
        ".github/ISSUE_TEMPLATE/agent-workflow-request.yml".into(),
        ".github/ISSUE_TEMPLATE/roadmap-request.yml".into(),
    ])
}

fn check_known_limitations_docs() -> Result<()> {
    check_known_limitations_docs_inner("docs/first-release.md".into())
}

fn check_security_assumptions_docs() -> Result<()> {
    check_security_assumptions_docs_inner("docs/first-release.md".into())
}

fn check_security_policy() -> Result<()> {
    check_security_policy_inner([
        "SECURITY.md".into(),
        "README.md".into(),
        ".github/ISSUE_TEMPLATE/config.yml".into(),
    ])
}

fn check_known_limitations_docs_inner(first_release_path: PathBuf) -> Result<()> {
    let docs = [DocExpectation {
        path: first_release_path,
        required_phrases: vec![
            "## Known Limitations".into(),
            "Kply `v0.1.0` is an evaluation release, not a production safety guarantee.".into(),
            "No automatic promotion".into(),
            "`kply route apply` is a no-op placeholder".into(),
            "Experimental live apply can create sandbox Deployment and Service resources".into(),
            "Preview Service and no-route checks do not prove edge routing behavior".into(),
            "Kply never reads Kubernetes Secret values".into(),
            "No hosted policy, team approval".into(),
            "No long-running in-cluster controller reconciles sessions after the CLI exits".into(),
            "Runtime checks report evidence, not approval to deploy".into(),
            "The JSON contracts are stable only for `v0.1.0` evaluation".into(),
            "The local demo is bounded to the Kind ecommerce fixture".into(),
        ],
    }];

    check_docs_contain(docs)
}

fn check_feedback_triage_docs_inner(feedback_triage_path: PathBuf) -> Result<()> {
    let docs = [DocExpectation {
        path: feedback_triage_path,
        required_phrases: vec![
            "# Feedback Triage".into(),
            "## Missing Route Adapters".into(),
            "## Policy Needs".into(),
            "missing route adapter".into(),
            "Three separate users or organizations".into(),
            "repeated route adapter request".into(),
            "repeated policy need".into(),
            "same policy boundary".into(),
            "minimum deterministic decision".into(),
            "Do not include Secret values".into(),
            "explicit permission exists".into(),
            "OpenSpec change".into(),
        ],
    }];

    check_docs_contain(docs)
}

fn check_issue_templates_inner(template_paths: [PathBuf; 6]) -> Result<()> {
    let [
        config_path,
        routing_path,
        discovery_path,
        planning_path,
        agent_path,
        roadmap_path,
    ] = template_paths;
    let docs = [
        DocExpectation {
            path: config_path,
            required_phrases: vec![
                "blank_issues_enabled: false".into(),
                "Security reports".into(),
                "/security/policy".into(),
            ],
        },
        DocExpectation {
            path: routing_path,
            required_phrases: vec![
                "name: Routing environment".into(),
                "labels:".into(),
                "routing".into(),
                "Gateway API implementation, ingress controller, service mesh".into(),
                "missing route adapter request".into(),
                "Adapter gap".into(),
                "Do not include Secret values".into(),
                "Route strategy".into(),
                "Sanitized Kply output".into(),
            ],
        },
        DocExpectation {
            path: discovery_path,
            required_phrases: vec![
                "name: Kubernetes discovery bug".into(),
                "discovery".into(),
                "Do not include Secret values".into(),
                "Workload shape".into(),
                "Secret metadata references".into(),
                "Expected discovery result".into(),
            ],
        },
        DocExpectation {
            path: planning_path,
            required_phrases: vec![
                "name: Session planning gap".into(),
                "session-planning".into(),
                "Do not include Secret values".into(),
                "Command and sanitized config".into(),
                "unsupported feature warnings and risk notes".into(),
                "repeated policy need".into(),
                "Policy gap".into(),
                "Safety constraints".into(),
            ],
        },
        DocExpectation {
            path: agent_path,
            required_phrases: vec![
                "name: Agent workflow request".into(),
                "agent-workflow".into(),
                "Codex, Claude Code, Cursor".into(),
                "Do not include Secret values".into(),
                "Failure mode to prevent".into(),
                "Useful Kply behavior".into(),
            ],
        },
        DocExpectation {
            path: roadmap_path,
            required_phrases: vec![
                "name: Roadmap request".into(),
                "roadmap".into(),
                "real user feedback, demos, benchmarks".into(),
                "Do not include Secret values".into(),
                "Evidence type".into(),
                "infra-bench failure".into(),
                "repeated policy need".into(),
                "Requested roadmap change".into(),
                "Affected agent-infra workflow".into(),
                "Success criteria".into(),
            ],
        },
    ];

    check_docs_contain(docs)
}

fn check_security_policy_inner(doc_paths: [PathBuf; 3]) -> Result<()> {
    let [security_path, readme_path, issue_config_path] = doc_paths;
    let docs = [
        DocExpectation {
            path: security_path,
            required_phrases: vec![
                "# Security Policy".into(),
                "Supported Versions".into(),
                "latest released `v0.1.x` version".into(),
                "GitHub private vulnerability reporting".into(),
                "Do not open a public issue for a vulnerability.".into(),
                "Kply reads or prints Kubernetes Secret values.".into(),
                "mutates resources without an explicit apply or confirmation boundary".into(),
                "Release artifacts, checksums, installers, or attestations".into(),
                "no paid bug bounty program".into(),
                "Do not send live credentials".into(),
            ],
        },
        DocExpectation {
            path: readme_path,
            required_phrases: vec![
                "## Security".into(),
                "SECURITY.md".into(),
                "private vulnerability reporting".into(),
            ],
        },
        DocExpectation {
            path: issue_config_path,
            required_phrases: vec![
                "Security reports".into(),
                "https://github.com/kubeply/kply/security/policy".into(),
                "private security advisory flow".into(),
            ],
        },
    ];

    check_docs_contain(docs)
}

fn check_security_assumptions_docs_inner(first_release_path: PathBuf) -> Result<()> {
    let docs = [DocExpectation {
        path: first_release_path,
        required_phrases: vec![
            "## Security Assumptions".into(),
            "Kply `v0.1.0` assumes the caller already has Kubernetes credentials".into(),
            "Kply does not bypass Kubernetes RBAC".into(),
            "Use a dedicated service account".into(),
            "Run Kply against a namespace the platform owner has approved".into(),
            "Admission policy should enforce ownership labels".into(),
            "Do not pass production admin kubeconfigs to agents".into(),
            "Network isolation, egress policy, and database permissions".into(),
            "Kply output is local CLI evidence and must be retained by the caller".into(),
            "Supply-chain trust depends on GitHub Release artifacts".into(),
            "Secret values remain out of scope".into(),
        ],
    }];

    check_docs_contain(docs)
}

fn check_future_session_docs_inner(doc_paths: [PathBuf; 3]) -> Result<()> {
    let [readme_path, architecture_path, product_path] = doc_paths;
    let docs = [
        DocExpectation {
            path: readme_path,
            required_phrases: vec![
                "Implementation in progress".into(),
                "early runtime check support".into(),
                "Session mutation commands require explicit `--apply` confirmation.".into(),
            ],
        },
        DocExpectation {
            path: architecture_path,
            required_phrases: vec![
                "Real session planning and Kubernetes execution are now implemented".into(),
                "Gateway API routing groundwork".into(),
            ],
        },
        DocExpectation {
            path: product_path,
            required_phrases: vec![
                "roadmap hypothesis, partially implemented behavior".into(),
                "runtime checks are landing".into(),
                "Gateway API routing groundwork".into(),
            ],
        },
    ];

    check_docs_contain(docs)
}

fn check_demo_docs_inner(doc_paths: [PathBuf; 4]) -> Result<()> {
    let [readme_path, kind_path, agent_path, fixture_path] = doc_paths;
    let docs = [
        DocExpectation {
            path: readme_path,
            required_phrases: vec!["## Local Demo".into(), "docs/demo-kind.md".into()],
        },
        DocExpectation {
            path: kind_path,
            required_phrases: vec![
                "# Local Kind Demo".into(),
                "demo-agent.md".into(),
                "scripts/demo-walkthrough.sh".into(),
                "demo doctor".into(),
                "demo install".into(),
                "demo reset".into(),
                "demo teardown".into(),
                "session create".into(),
                "check run".into(),
                "session cleanup".into(),
                "kind create cluster --name kply-demo".into(),
                "fixtures/demo/ecommerce-basic/kply.yaml".into(),
                "kply-demo".into(),
            ],
        },
        DocExpectation {
            path: agent_path,
            required_phrases: vec![
                "# Coding Agent Demo Guide".into(),
                "Use Kubernetes context kind-kply-demo only.".into(),
                "Do not touch resources outside the kply-demo namespace.".into(),
                "Do not read Kubernetes Secret values.".into(),
                "kply session create --apply".into(),
                "kply check run".into(),
                "kply session cleanup --apply".into(),
                "fixtures/demo/ecommerce-basic/manifests/".into(),
                "scripts/demo-walkthrough.sh".into(),
                "status: \"not_implemented\"".into(),
                "It does not yet prove live sandbox routing".into(),
            ],
        },
        DocExpectation {
            path: fixture_path,
            required_phrases: vec![
                "docs/demo-kind.md".into(),
                "docs/demo-agent.md".into(),
                "scripts/demo-walkthrough.sh".into(),
                "backend-broken.yaml".into(),
                "backend-fixed.yaml".into(),
            ],
        },
    ];

    check_docs_contain(docs)
}

fn check_release_planning() -> Result<()> {
    check_release_planning_inner(
        "dist-workspace.toml".as_ref(),
        ".github/workflows/release.yml".as_ref(),
    )
}

fn check_readme_roadmap_link() -> Result<()> {
    check_readme_roadmap_link_inner("README.md".as_ref())
}

fn check_toolchain_pin() -> Result<()> {
    check_toolchain_pin_inner(
        "rust-toolchain.toml".as_ref(),
        ".github/workflows/ci.yml".as_ref(),
        expected_rust_channel(),
    )
}

fn expected_rust_channel() -> &'static str {
    "1.95.0"
}

fn required_fixture_directories() -> &'static [&'static str] {
    &[
        "cli",
        "config",
        "manifests",
        "k8s-responses",
        "reports",
        "demo",
    ]
}

fn required_rust_components() -> &'static [&'static str] {
    &["clippy", "rustfmt"]
}

fn forbidden_secret_content_patterns() -> &'static [&'static str] {
    &[
        "api::core::v1::Secret",
        "core::v1::{Secret",
        "Api<Secret",
        "Api::<Secret",
        "Secret::",
    ]
}

fn secret_content_guard_source_roots() -> &'static [&'static str] {
    &[
        "crates/kply-checks/src",
        "crates/kply-cli/src",
        "crates/kply-config/src",
        "crates/kply-core/src",
        "crates/kply-k8s/src",
        "crates/kply-routing/src",
    ]
}

fn forbidden_report_overclaim_phrases() -> &'static [&'static str] {
    &[
        "safe to deploy",
        "production ready",
        "approved for promotion",
        "ready for promotion",
        "ready for approval or promotion",
    ]
}

fn report_language_paths() -> &'static [&'static str] {
    &[
        "README.md",
        "docs/architecture.md",
        "docs/cli.md",
        "docs/product.md",
        "docs/report-agent.md",
        "crates/kply-cli/src/main.rs",
        "crates/kply-core/src/lib.rs",
    ]
}

#[derive(Debug, Clone, Copy)]
struct WorkspaceCrate {
    name: &'static str,
    path: &'static str,
}

fn workspace_crates() -> &'static [WorkspaceCrate] {
    &[
        WorkspaceCrate {
            name: "kply-checks",
            path: "crates/kply-checks",
        },
        WorkspaceCrate {
            name: "kply-cli",
            path: "crates/kply-cli",
        },
        WorkspaceCrate {
            name: "kply-config",
            path: "crates/kply-config",
        },
        WorkspaceCrate {
            name: "kply-core",
            path: "crates/kply-core",
        },
        WorkspaceCrate {
            name: "kply-k8s",
            path: "crates/kply-k8s",
        },
        WorkspaceCrate {
            name: "kply-routing",
            path: "crates/kply-routing",
        },
        WorkspaceCrate {
            name: "kply-test",
            path: "crates/kply-test",
        },
        WorkspaceCrate {
            name: "xtask",
            path: "crates/xtask",
        },
    ]
}

fn check_crate_inventory_docs_inner(
    manifest_path: &Path,
    doc_paths: impl IntoIterator<Item = impl AsRef<Path>>,
    crates: &[WorkspaceCrate],
) -> Result<()> {
    let manifest_source = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("reading workspace manifest {}", manifest_path.display()))?;
    let workspace_members = collect_workspace_members(&manifest_source)?;
    let expected_members = crates
        .iter()
        .map(|workspace_crate| workspace_crate.path)
        .collect::<Vec<_>>();
    let workspace_member_set = workspace_members
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected_member_set = expected_members.iter().copied().collect::<BTreeSet<_>>();

    if workspace_member_set != expected_member_set {
        let missing_members = expected_member_set
            .difference(&workspace_member_set)
            .copied()
            .collect::<Vec<_>>();
        let unexpected_members = workspace_member_set
            .difference(&expected_member_set)
            .copied()
            .collect::<Vec<_>>();
        bail!(
            "workspace crate inventory does not match Cargo.toml members: missing {:?}, unexpected {:?}",
            missing_members,
            unexpected_members
        );
    }

    let mut missing_entries = Vec::new();

    for doc_path in doc_paths {
        let doc_path = doc_path.as_ref();
        let source = std::fs::read_to_string(doc_path)
            .with_context(|| format!("reading crate inventory doc {}", doc_path.display()))?;

        for workspace_crate in crates {
            if !contains_crate_name(&source, workspace_crate.name) {
                missing_entries.push((doc_path.to_path_buf(), workspace_crate.name));
            }
        }
    }

    if !missing_entries.is_empty() {
        for (doc_path, crate_name) in &missing_entries {
            eprintln!(
                "crate inventory entry missing in {}: {crate_name}",
                doc_path.display()
            );
        }
        bail!("{} crate inventory entries missing", missing_entries.len());
    }

    Ok(())
}

fn check_license_files_inner(
    license_path: &Path,
    notice_path: &Path,
    manifest_path: &Path,
    crates: &[WorkspaceCrate],
) -> Result<()> {
    let mut errors = Vec::new();
    let license_source = std::fs::read_to_string(license_path)
        .with_context(|| format!("reading license file {}", license_path.display()))?;
    let notice_source = std::fs::read_to_string(notice_path)
        .with_context(|| format!("reading notice file {}", notice_path.display()))?;

    for phrase in [
        "Apache License",
        "Version 2.0, January 2004",
        "http://www.apache.org/licenses/",
    ] {
        if !license_source.contains(phrase) {
            errors.push(format!("LICENSE is missing Apache-2.0 phrase: {phrase}"));
        }
    }

    for phrase in [
        "Kply",
        "Copyright 2026 Kubeply",
        "software developed by Kubeply",
    ] {
        if !notice_source.contains(phrase) {
            errors.push(format!("NOTICE is missing required phrase: {phrase}"));
        }
    }

    let manifest = parse_toml_file(manifest_path)?;
    let workspace_license = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("license"))
        .and_then(toml::Value::as_str);

    if workspace_license != Some("Apache-2.0") {
        errors.push("workspace package license must be Apache-2.0".to_owned());
    }

    let workspace_root = manifest_path.parent().unwrap_or_else(|| Path::new("."));

    for workspace_crate in crates {
        let crate_manifest_path = workspace_root.join(workspace_crate.path).join("Cargo.toml");
        let crate_manifest = parse_toml_file(&crate_manifest_path)?;
        let inherits_workspace_license = crate_manifest
            .get("package")
            .and_then(|package| package.get("license"))
            .and_then(|license| license.get("workspace"))
            .and_then(toml::Value::as_bool)
            == Some(true);

        if !inherits_workspace_license {
            errors.push(format!(
                "{} must inherit license.workspace = true",
                crate_manifest_path.display()
            ));
        }
    }

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error}");
        }
        bail!("{} license file issue(s) found", errors.len());
    }

    Ok(())
}

fn check_deny_config_inner(deny_path: &Path) -> Result<()> {
    let deny_config = parse_toml_file(deny_path)?;
    let mut errors = Vec::new();

    let yanked = deny_config
        .get("advisories")
        .and_then(|advisories| advisories.get("yanked"))
        .and_then(toml::Value::as_str);

    if yanked != Some("deny") {
        errors.push("advisories.yanked must be deny".to_owned());
    }

    let ignore_is_empty = deny_config
        .get("advisories")
        .and_then(|advisories| advisories.get("ignore"))
        .and_then(toml::Value::as_array)
        .is_some_and(Vec::is_empty);

    if !ignore_is_empty {
        errors.push("advisories.ignore must stay empty".to_owned());
    }

    let allowed_licenses = deny_config
        .get("licenses")
        .and_then(|licenses| licenses.get("allow"))
        .and_then(toml::Value::as_array)
        .map(|licenses| {
            licenses
                .iter()
                .filter_map(toml::Value::as_str)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let expected_licenses = ["Apache-2.0", "MIT", "Unicode-3.0"]
        .into_iter()
        .collect::<BTreeSet<_>>();

    if allowed_licenses != expected_licenses {
        errors.push(format!(
            "licenses.allow must be exactly {:?}",
            expected_licenses
        ));
    }

    let confidence_threshold = deny_config
        .get("licenses")
        .and_then(|licenses| licenses.get("confidence-threshold"))
        .and_then(toml::Value::as_float);

    if confidence_threshold.is_none_or(|threshold| threshold < 0.8) {
        errors.push("licenses.confidence-threshold must be at least 0.8".to_owned());
    }

    for (key, expected) in [("multiple-versions", "deny"), ("wildcards", "allow")] {
        let actual = deny_config
            .get("bans")
            .and_then(|bans| bans.get(key))
            .and_then(toml::Value::as_str);

        if actual != Some(expected) {
            errors.push(format!("bans.{key} must be {expected}"));
        }
    }

    let highlight = deny_config
        .get("bans")
        .and_then(|bans| bans.get("highlight"))
        .and_then(toml::Value::as_str);

    if highlight != Some("all") {
        errors.push("bans.highlight must be all".to_owned());
    }

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error}");
        }
        bail!("{} cargo-deny config issue(s) found", errors.len());
    }

    Ok(())
}

fn check_release_planning_inner(dist_path: &Path, release_workflow_path: &Path) -> Result<()> {
    let dist_config = parse_toml_file(dist_path)?;
    let release_workflow = std::fs::read_to_string(release_workflow_path).with_context(|| {
        format!(
            "reading release workflow {}",
            release_workflow_path.display()
        )
    })?;
    let release_workflow_yaml: YamlValue =
        serde_norway::from_str(&release_workflow).with_context(|| {
            format!(
                "parsing release workflow {}",
                release_workflow_path.display()
            )
        })?;
    let mut errors = Vec::new();

    let cargo_dist_version = dist_config
        .get("dist")
        .and_then(|dist| dist.get("cargo-dist-version"))
        .and_then(toml::Value::as_str);

    if cargo_dist_version != Some("0.32.0") {
        errors.push("dist.cargo-dist-version must stay pinned to 0.32.0".to_owned());
    }

    let pr_run_mode = dist_config
        .get("dist")
        .and_then(|dist| dist.get("pr-run-mode"))
        .and_then(toml::Value::as_str);

    if pr_run_mode != Some("plan") {
        errors.push("dist.pr-run-mode must stay plan".to_owned());
    }

    let allow_dirty = dist_config
        .get("dist")
        .and_then(|dist| dist.get("allow-dirty"))
        .and_then(toml::Value::as_array);

    if !matches!(
        allow_dirty,
        Some(allow_dirty)
            if allow_dirty.len() == 1
                && allow_dirty.first().and_then(toml::Value::as_str) == Some("ci")
    ) {
        errors.push("dist.allow-dirty must allow only ci workflow permission hardening".to_owned());
    }

    let packages = dist_config
        .get("dist")
        .and_then(|dist| dist.get("packages"))
        .and_then(toml::Value::as_array);

    if !matches!(
        packages,
        Some(packages)
            if packages.len() == 1
                && packages.first().and_then(toml::Value::as_str) == Some("kply-cli")
    ) {
        errors.push("dist.packages must release only kply-cli".to_owned());
    }

    let installers = dist_config
        .get("dist")
        .and_then(|dist| dist.get("installers"))
        .and_then(toml::Value::as_array);

    if !matches!(
        installers,
        Some(installers)
            if installers.len() == 1
                && installers.first().and_then(toml::Value::as_str) == Some("shell")
    ) {
        errors.push("dist.installers must generate the shell installer only".to_owned());
    }

    let checksum = dist_config
        .get("dist")
        .and_then(|dist| dist.get("checksum"))
        .and_then(toml::Value::as_str);

    if checksum != Some("sha256") {
        errors.push("dist.checksum must generate sha256 checksums".to_owned());
    }

    let github_attestations = dist_config
        .get("dist")
        .and_then(|dist| dist.get("github-attestations"))
        .and_then(toml::Value::as_bool);

    if github_attestations != Some(true) {
        errors.push("dist.github-attestations must stay enabled".to_owned());
    }

    let targets = dist_config
        .get("dist")
        .and_then(|dist| dist.get("targets"))
        .and_then(toml::Value::as_array);

    if !toml_array_contains_str(targets, "x86_64-unknown-linux-gnu") {
        errors.push("dist.targets must include x86_64-unknown-linux-gnu".to_owned());
    }

    if !toml_array_contains_str(targets, "aarch64-unknown-linux-gnu") {
        errors.push("dist.targets must include aarch64-unknown-linux-gnu".to_owned());
    }

    if !toml_array_contains_str(targets, "x86_64-unknown-linux-musl") {
        errors.push("dist.targets must include x86_64-unknown-linux-musl".to_owned());
    }

    if !toml_array_contains_str(targets, "aarch64-unknown-linux-musl") {
        errors.push("dist.targets must include aarch64-unknown-linux-musl".to_owned());
    }

    if !toml_array_contains_str(targets, "x86_64-apple-darwin") {
        errors.push("dist.targets must include x86_64-apple-darwin".to_owned());
    }

    if !toml_array_contains_str(targets, "aarch64-apple-darwin") {
        errors.push("dist.targets must include aarch64-apple-darwin".to_owned());
    }

    if !workflow_has_pull_request(&release_workflow_yaml) {
        errors.push("release workflow must run on pull_request".to_owned());
    }

    if workflow_has_push_tags(&release_workflow_yaml) {
        // Tag pushes are the real release path once release packaging starts.
    } else {
        errors.push("release workflow must run on semver tag pushes".to_owned());
    }

    let run_commands = workflow_run_commands(&release_workflow_yaml);

    for forbidden in ["dist publish"] {
        if run_commands
            .iter()
            .any(|run_command| run_command.contains(forbidden))
        {
            errors.push(format!(
                "release workflow must not contain publishing command: {forbidden}"
            ));
        }
    }

    if !run_commands
        .iter()
        .any(|run_command| run_command.contains("dist plan") || run_command.contains("|| 'plan'"))
    {
        errors.push("release workflow must keep dist plan command".to_owned());
    }

    if !run_commands
        .iter()
        .any(|run_command| run_command.contains("dist build"))
    {
        errors.push("release workflow must build release artifacts".to_owned());
    }

    if !run_commands
        .iter()
        .any(|run_command| run_command.contains("dist host"))
    {
        errors.push("release workflow must host release artifacts".to_owned());
    }

    if !run_commands
        .iter()
        .any(|run_command| run_command.contains("gh release create"))
    {
        errors.push("release workflow must create a GitHub Release".to_owned());
    }

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error}");
        }
        bail!(
            "{} release planning issue(s) found: {}",
            errors.len(),
            errors.join("; ")
        );
    }

    Ok(())
}

fn check_ci_workflow_inner(workflow_path: &Path) -> Result<()> {
    let workflow_source = std::fs::read_to_string(workflow_path)
        .with_context(|| format!("reading workflow file {}", workflow_path.display()))?;
    let workflow_yaml: YamlValue = serde_norway::from_str(&workflow_source)
        .with_context(|| format!("parsing workflow file {}", workflow_path.display()))?;
    let mut errors = Vec::new();

    if !workflow_has_pull_request(&workflow_yaml) {
        errors.push("ci workflow must run on pull_request".to_owned());
    }

    if !workflow_has_merge_group(&workflow_yaml) {
        errors.push("ci workflow must run on merge_group".to_owned());
    }

    if !workflow_pushes_branch(&workflow_yaml, "main") {
        errors.push("ci workflow must run on pushes to main".to_owned());
    }

    if !workflow_has_read_only_contents_permission(&workflow_yaml) {
        errors.push("ci workflow must keep contents: read permission".to_owned());
    }

    let run_commands = workflow_run_commands(&workflow_yaml);
    let run_lines = run_commands
        .iter()
        .flat_map(|run_command| run_command.lines())
        .map(str::trim)
        .collect::<BTreeSet<_>>();
    for required_command in required_ci_run_commands() {
        if !run_lines.contains(required_command) {
            errors.push(format!(
                "ci workflow must run required command: {required_command}"
            ));
        }
    }

    let uses = workflow_uses_actions(&workflow_yaml);
    let used_action_repositories = uses
        .iter()
        .map(|action| {
            action
                .split_once('@')
                .map_or(*action, |(repository, _)| repository)
        })
        .map(str::trim)
        .collect::<BTreeSet<_>>();
    for required_action in [
        "raven-actions/actionlint",
        "dtolnay/rust-toolchain",
        "EmbarkStudios/cargo-deny-action",
    ] {
        if !used_action_repositories.contains(required_action) {
            errors.push(format!(
                "ci workflow must use required action: {required_action}"
            ));
        }
    }

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error}");
        }
        bail!(
            "{} ci workflow issue(s) found: {}",
            errors.len(),
            errors.join("; ")
        );
    }

    Ok(())
}

fn workflow_has_pull_request(workflow: &YamlValue) -> bool {
    workflow_event(workflow, &YAML_PULL_REQUEST_KEY).is_some()
}

fn workflow_has_merge_group(workflow: &YamlValue) -> bool {
    workflow_event(workflow, &YAML_MERGE_GROUP_KEY).is_some()
}

fn workflow_pushes_branch(workflow: &YamlValue, expected_branch: &str) -> bool {
    workflow_event(workflow, &YAML_PUSH_KEY)
        .and_then(YamlValue::as_mapping)
        .and_then(|push| push.get(&*YAML_BRANCHES_KEY))
        .and_then(YamlValue::as_sequence)
        .is_some_and(|branches| {
            branches
                .iter()
                .filter_map(YamlValue::as_str)
                .any(|branch| branch == expected_branch)
        })
}

fn workflow_has_read_only_contents_permission(workflow: &YamlValue) -> bool {
    let top_level_read_only = workflow
        .as_mapping()
        .and_then(|workflow| workflow.get(&*YAML_PERMISSIONS_KEY))
        .and_then(YamlValue::as_mapping)
        .and_then(|permissions| permissions.get(&*YAML_CONTENTS_KEY))
        .and_then(YamlValue::as_str)
        == Some("read");

    let jobs_do_not_escalate = workflow
        .as_mapping()
        .and_then(|workflow| workflow.get(&*YAML_JOBS_KEY))
        .and_then(YamlValue::as_mapping)
        .is_none_or(|jobs| {
            jobs.values().filter_map(YamlValue::as_mapping).all(|job| {
                job.get(&*YAML_PERMISSIONS_KEY)
                    .and_then(YamlValue::as_mapping)
                    .and_then(|permissions| permissions.get(&*YAML_CONTENTS_KEY))
                    .and_then(YamlValue::as_str)
                    .is_none_or(|contents| contents == "read")
            })
        });

    top_level_read_only && jobs_do_not_escalate
}

fn workflow_has_push_tags(workflow: &YamlValue) -> bool {
    workflow_event(workflow, &YAML_PUSH_KEY)
        .and_then(YamlValue::as_mapping)
        .and_then(|push| push.get(&*YAML_TAGS_KEY))
        .and_then(YamlValue::as_sequence)
        .is_some_and(|tags| {
            tags.iter()
                .filter_map(YamlValue::as_str)
                .any(tag_filter_has_semver_shape)
        })
}

fn tag_filter_has_semver_shape(tag_filter: &str) -> bool {
    tag_filter.contains("[0-9]+.[0-9]+.[0-9]+")
}

fn required_ci_run_commands() -> &'static [&'static str] {
    &[
        "cargo fmt --all -- --check",
        "cargo check --all-targets --all-features --locked",
        "cargo clippy --all-targets --all-features --locked -- -D warnings",
        "cargo test --all-targets --all-features --locked",
        "cargo test -p kply-test --locked",
        "cargo xtask check-ci-workflow",
        "cargo xtask check-crate-inventory-docs",
        "cargo xtask check-demo-docs",
        "cargo xtask check-deny-config",
        "cargo xtask check-fixture-directories",
        "cargo xtask check-fixture-naming-docs",
        "cargo xtask check-fixture-testing-docs",
        "cargo xtask check-feedback-triage-docs",
        "cargo xtask check-future-session-docs",
        "cargo xtask check-issue-templates",
        "cargo xtask check-known-limitations-docs",
        "cargo xtask check-license-files",
        "cargo xtask check-module-docs",
        "cargo xtask check-no-secret-content-reads",
        "cargo xtask check-placeholder-docs",
        "cargo xtask check-placeholders",
        "cargo xtask check-report-language",
        "cargo xtask check-readme-roadmap-link",
        "cargo xtask check-release-planning",
        "cargo xtask check-security-assumptions-docs",
        "cargo xtask check-security-policy",
        "cargo xtask check-toolchain-pin",
    ]
}

fn toml_array_contains_str(values: Option<&Vec<toml::Value>>, expected: &str) -> bool {
    values.is_some_and(|values| {
        values
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == expected)
    })
}

fn workflow_event<'a>(workflow: &'a YamlValue, event_key: &YamlValue) -> Option<&'a YamlValue> {
    workflow
        .as_mapping()
        .and_then(|workflow| workflow.get(&*YAML_ON_KEY))
        .and_then(YamlValue::as_mapping)
        .and_then(|events| events.get(event_key))
}

fn workflow_run_commands(workflow: &YamlValue) -> Vec<&str> {
    let Some(jobs) = workflow
        .as_mapping()
        .and_then(|workflow| workflow.get(&*YAML_JOBS_KEY))
        .and_then(YamlValue::as_mapping)
    else {
        return Vec::new();
    };

    jobs.values()
        .filter_map(YamlValue::as_mapping)
        .filter_map(|job| job.get(&*YAML_STEPS_KEY))
        .filter_map(YamlValue::as_sequence)
        .flat_map(|steps| steps.iter())
        .filter_map(YamlValue::as_mapping)
        .filter_map(|step| step.get(&*YAML_RUN_KEY))
        .filter_map(YamlValue::as_str)
        .collect()
}

fn workflow_uses_actions(workflow: &YamlValue) -> Vec<&str> {
    let Some(jobs) = workflow
        .as_mapping()
        .and_then(|workflow| workflow.get(&*YAML_JOBS_KEY))
        .and_then(YamlValue::as_mapping)
    else {
        return Vec::new();
    };

    jobs.values()
        .filter_map(YamlValue::as_mapping)
        .filter_map(|job| job.get(&*YAML_STEPS_KEY))
        .filter_map(YamlValue::as_sequence)
        .flat_map(|steps| steps.iter())
        .filter_map(YamlValue::as_mapping)
        .filter_map(|step| step.get(&*YAML_USES_KEY))
        .filter_map(YamlValue::as_str)
        .collect()
}

fn parse_toml_file(path: &Path) -> Result<toml::Value> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading TOML file {}", path.display()))?;
    toml::from_str(&source).with_context(|| format!("parsing TOML file {}", path.display()))
}

fn check_toolchain_pin_inner(
    toolchain_path: &Path,
    workflow_path: &Path,
    expected_channel: &str,
) -> Result<()> {
    let mut errors = Vec::new();
    let toolchain = parse_toml_file(toolchain_path)?;
    let channel = toolchain
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("channel"))
        .and_then(toml::Value::as_str);

    if channel != Some(expected_channel) {
        errors.push(format!(
            "{} must pin channel = \"{}\"",
            toolchain_path.display(),
            expected_channel
        ));
    }

    let components = toolchain
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("components"))
        .and_then(toml::Value::as_array)
        .map(|components| {
            components
                .iter()
                .filter_map(toml::Value::as_str)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    for component in required_rust_components() {
        if !components.contains(component) {
            errors.push(format!(
                "{} must include Rust component {component}",
                toolchain_path.display()
            ));
        }
    }

    let workflow_source = std::fs::read_to_string(workflow_path)
        .with_context(|| format!("reading workflow file {}", workflow_path.display()))?;

    if !workflow_installs_toolchain(&workflow_source, expected_channel) {
        errors.push(format!(
            "{} must install Rust toolchain {expected_channel}",
            workflow_path.display()
        ));
    }

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error}");
        }
        bail!("{} toolchain pin issue(s) found", errors.len());
    }

    Ok(())
}

fn check_no_secret_content_reads_inner(
    source_paths: impl IntoIterator<Item = impl AsRef<Path>>,
    forbidden_patterns: &[&str],
) -> Result<()> {
    let mut matches = Vec::new();

    for source_path in source_paths {
        let source_path = source_path.as_ref();
        let source = std::fs::read_to_string(source_path)
            .with_context(|| format!("reading source file {}", source_path.display()))?;
        let sanitized_lines = strip_rust_comments_and_strings(&source);
        let secret_identifiers = sanitized_lines
            .iter()
            .flat_map(|line| SECRET_TYPED_IDENTIFIER_RE.captures_iter(line))
            .filter_map(|capture| capture.get(1).map(|identifier| identifier.as_str()))
            .collect::<BTreeSet<_>>();

        for (line_index, (line, sanitized_line)) in
            source.lines().zip(sanitized_lines.iter()).enumerate()
        {
            for pattern in forbidden_patterns {
                if sanitized_line.contains(pattern) {
                    matches.push((
                        source_path.to_path_buf(),
                        line_index + 1,
                        (*pattern).to_owned(),
                        line.trim().to_owned(),
                    ));
                }
            }

            for capture in SECRET_FIELD_ACCESS_RE.captures_iter(sanitized_line) {
                let Some(identifier) = capture.get(1).map(|identifier| identifier.as_str()) else {
                    continue;
                };
                if identifier.to_ascii_lowercase().contains("secret")
                    || secret_identifiers.contains(identifier)
                {
                    let field = capture
                        .get(2)
                        .map_or("Secret content field".to_owned(), |field| {
                            format!(".{}", field.as_str())
                        });
                    matches.push((
                        source_path.to_path_buf(),
                        line_index + 1,
                        field,
                        line.trim().to_owned(),
                    ));
                }
            }
        }
    }

    if !matches.is_empty() {
        for (source_path, line, pattern, source_line) in &matches {
            eprintln!(
                "forbidden Secret content access pattern in {}:{line}: {pattern}: {source_line}",
                source_path.display()
            );
        }
        bail!(
            "{} forbidden Secret content access pattern(s) found",
            matches.len()
        );
    }

    Ok(())
}

fn check_report_language_inner(
    source_paths: impl IntoIterator<Item = impl AsRef<Path>>,
    forbidden_phrases: &[&str],
) -> Result<()> {
    let mut violations = Vec::new();

    for source_path in source_paths {
        let source_path = source_path.as_ref();
        let source = std::fs::read_to_string(source_path)
            .with_context(|| format!("reading report language source {}", source_path.display()))?;

        for (line_index, source_line) in source.lines().enumerate() {
            let line = line_index + 1;
            let normalized_line = source_line.to_ascii_lowercase();

            for phrase in forbidden_phrases {
                if normalized_line.contains(phrase) {
                    violations.push((
                        source_path.to_path_buf(),
                        line,
                        (*phrase).to_owned(),
                        source_line.trim().to_owned(),
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        for (source_path, line, phrase, source_line) in &violations {
            eprintln!(
                "report language overclaim in {}:{line}: {phrase}: {source_line}",
                source_path.display()
            );
        }
        bail!("{} report language overclaim(s) found", violations.len());
    }

    Ok(())
}

fn strip_rust_comments_and_strings(source: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut sanitized = String::new();
    let chars = source.chars().collect::<Vec<_>>();
    let mut index = 0;

    while index < chars.len() {
        let character = chars[index];

        if character == '\n' {
            lines.push(std::mem::take(&mut sanitized));
            index += 1;
            continue;
        }

        if character == '/' && chars.get(index + 1) == Some(&'/') {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }

        if character == '/' && chars.get(index + 1) == Some(&'*') {
            sanitized.push(' ');
            sanitized.push(' ');
            index += 2;
            while index < chars.len() {
                if chars[index] == '\n' {
                    lines.push(std::mem::take(&mut sanitized));
                    index += 1;
                } else if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    sanitized.push(' ');
                    sanitized.push(' ');
                    index += 2;
                    break;
                } else {
                    sanitized.push(' ');
                    index += 1;
                }
            }
            continue;
        }

        if let Some(raw_string_hashes) = raw_string_hashes(&chars, index) {
            sanitized.push(' ');
            index += 1;
            for _ in 0..raw_string_hashes {
                sanitized.push(' ');
                index += 1;
            }
            sanitized.push(' ');
            index += 1;

            while index < chars.len() {
                if chars[index] == '\n' {
                    lines.push(std::mem::take(&mut sanitized));
                    index += 1;
                    continue;
                }
                sanitized.push(' ');
                if chars[index] == '"'
                    && (0..raw_string_hashes)
                        .all(|offset| chars.get(index + 1 + offset) == Some(&'#'))
                {
                    index += 1;
                    for _ in 0..raw_string_hashes {
                        sanitized.push(' ');
                        index += 1;
                    }
                    break;
                }
                index += 1;
            }
            continue;
        }

        if character == '"' {
            sanitized.push(' ');
            index += 1;
            while index < chars.len() {
                if chars[index] == '\n' {
                    lines.push(std::mem::take(&mut sanitized));
                    index += 1;
                    break;
                }
                sanitized.push(' ');
                if chars[index] == '\\' {
                    index += 2;
                } else if chars[index] == '"' {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
        } else if character == '\'' {
            sanitized.push(' ');
            index += 1;

            if chars
                .get(index)
                .is_some_and(|next| next.is_ascii_alphabetic() || *next == '_')
            {
                let identifier_start = index;
                while chars
                    .get(index)
                    .is_some_and(|next| next.is_ascii_alphanumeric() || *next == '_')
                {
                    sanitized.push(' ');
                    index += 1;
                }

                if index == identifier_start + 1 && chars.get(index) == Some(&'\'') {
                    sanitized.push(' ');
                    index += 1;
                }
            } else if chars.get(index) == Some(&'\\') {
                sanitized.push(' ');
                index += 1;
                if index < chars.len() {
                    sanitized.push(' ');
                    index += 1;
                }
                if chars.get(index) == Some(&'\'') {
                    sanitized.push(' ');
                    index += 1;
                }
            } else {
                while index < chars.len() {
                    sanitized.push(' ');
                    if chars[index] == '\'' {
                        index += 1;
                        break;
                    }
                    index += 1;
                }
            }
        } else {
            sanitized.push(character);
            index += 1;
        }
    }

    if !sanitized.is_empty() || source.ends_with('\n') {
        lines.push(sanitized);
    }

    lines
}

fn raw_string_hashes(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'r') {
        return None;
    }

    let mut index = start + 1;
    while chars.get(index) == Some(&'#') {
        index += 1;
    }

    (chars.get(index) == Some(&'"')).then_some(index - start - 1)
}

/// Returns true when any workflow line pins `toolchain:` to `expected_channel`.
///
/// This intentionally checks presence, not uniqueness: if `workflow_source`
/// contains conflicting toolchain lines, this still accepts as long as one line
/// matches. Tighten this if CI starts installing Rust in multiple jobs.
fn workflow_installs_toolchain(workflow_source: &str, expected_channel: &str) -> bool {
    workflow_source.lines().any(|line| {
        let line = line.trim();
        line.strip_prefix("toolchain:")
            .is_some_and(|value| value.trim() == expected_channel)
    })
}

fn collect_workspace_members(manifest_source: &str) -> Result<Vec<String>> {
    let manifest: toml::Value =
        toml::from_str(manifest_source).context("parsing workspace manifest TOML")?;
    let Some(members) = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
    else {
        return Ok(Vec::new());
    };

    members
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .context("workspace member must be a string")
        })
        .collect()
}

fn contains_crate_name(source: &str, crate_name: &str) -> bool {
    source.match_indices(crate_name).any(|(start, _)| {
        let before = source[..start].chars().next_back();
        let after = source[start + crate_name.len()..].chars().next();

        !is_crate_name_character(before) && !is_crate_name_character(after)
    })
}

fn is_crate_name_character(character: Option<char>) -> bool {
    character.is_some_and(|character| {
        character.is_ascii_alphanumeric() || character == '-' || character == '_'
    })
}

fn collect_crate_sources(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut source_paths = Vec::new();
    collect_crate_sources_inner(root.as_ref(), &mut source_paths)?;
    source_paths.sort();
    Ok(source_paths)
}

fn check_placeholders() -> Result<()> {
    // Product crates that have reached their roadmap work no longer need placeholder-only guards.
    let product_crates: [&str; 0] = [];

    check_placeholder_sources(product_crates)
}

fn check_placeholder_sources(
    source_paths: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<()> {
    let mut invalid_sources = Vec::new();

    for source_path in source_paths {
        let source_path = source_path.as_ref();
        let source = std::fs::read_to_string(source_path)?;

        if !has_placeholder_marker(&source) || has_non_placeholder_public_item(&source) {
            invalid_sources.push(source_path.to_path_buf());
        }
    }

    if !invalid_sources.is_empty() {
        for source_path in &invalid_sources {
            eprintln!(
                "product crate is not placeholder-only: {}",
                source_path.display()
            );
        }
        let invalid_source_list = invalid_sources
            .iter()
            .map(|source_path| source_path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "{} product crate source file(s) are not placeholder-only: {}",
            invalid_sources.len(),
            invalid_source_list
        );
    }

    Ok(())
}

fn check_placeholder_docs() -> Result<()> {
    let docs = [
        DocExpectation {
            path: "README.md".into(),
            required_phrases: vec![
                "Implementation in progress".into(),
                "Gateway API routing groundwork".into(),
            ],
        },
        DocExpectation {
            path: "docs/architecture.md".into(),
            required_phrases: vec![
                "kply CLI".into(),
                "Real session planning and Kubernetes execution".into(),
            ],
        },
        DocExpectation {
            path: "docs/product.md".into(),
            required_phrases: vec![
                "roadmap hypothesis, partially implemented behavior".into(),
                "Gateway API routing groundwork".into(),
            ],
        },
    ];

    check_docs_contain(docs)
}

struct DocExpectation {
    path: PathBuf,
    required_phrases: Vec<String>,
}

fn check_docs_contain(docs: impl IntoIterator<Item = DocExpectation>) -> Result<()> {
    let mut missing_phrases = Vec::new();

    for doc in docs {
        let source = std::fs::read_to_string(&doc.path)
            .with_context(|| format!("reading documentation file {}", doc.path.display()))?;

        let missing_for_doc: Vec<_> = doc
            .required_phrases
            .into_iter()
            .filter(|phrase| !source.contains(phrase))
            .collect();

        if !missing_for_doc.is_empty() {
            missing_phrases.push((doc.path, missing_for_doc));
        }
    }

    if !missing_phrases.is_empty() {
        let phrase_count: usize = missing_phrases
            .iter()
            .map(|(_, phrases)| phrases.len())
            .sum();

        for (path, phrases) in &missing_phrases {
            for phrase in phrases {
                eprintln!(
                    "placeholder documentation phrase missing in {}: {phrase}",
                    path.display()
                );
            }
        }
        bail!("{phrase_count} placeholder documentation phrase(s) missing");
    }

    Ok(())
}

fn check_readme_roadmap_link_inner(readme_path: &Path) -> Result<()> {
    let source = std::fs::read_to_string(readme_path)
        .with_context(|| format!("reading README file {}", readme_path.display()))?;
    let has_roadmap_heading = markdown_has_heading_outside_code_block(&source, "## Roadmap");
    let has_roadmap_link =
        source.contains("[docs/implementation-roadmap.md](docs/implementation-roadmap.md)");

    let mut errors = Vec::new();

    if !has_roadmap_heading {
        errors.push("missing top-level ## Roadmap heading");
    }

    if !has_roadmap_link {
        errors.push("missing markdown link to docs/implementation-roadmap.md");
    }

    if !errors.is_empty() {
        bail!(
            "{} must include a top-level Roadmap section linking docs/implementation-roadmap.md: {}",
            readme_path.display(),
            errors.join("; ")
        );
    }

    Ok(())
}

fn check_fixture_directories_inner(
    fixtures_root: &Path,
    required_directories: &[&str],
) -> Result<()> {
    let mut missing_directories = Vec::new();

    for directory in required_directories {
        let path = fixtures_root.join(directory);

        if !path.is_dir() {
            missing_directories.push(path);
        }
    }

    if !missing_directories.is_empty() {
        let missing_list = missing_directories
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "{} fixture directories missing: {}",
            missing_directories.len(),
            missing_list
        );
    }

    Ok(())
}

fn check_fixture_naming_docs_inner(fixture_readme_path: &Path) -> Result<()> {
    check_docs_contain([DocExpectation {
        path: fixture_readme_path.into(),
        required_phrases: vec![
            "cli/<behavior-name>/".into(),
            "config/<case-name>/kply.yaml".into(),
            "manifests/<workload-shape>/".into(),
            "k8s-responses/<api-shape>/".into(),
            "reports/<workflow-name>/".into(),
            "demo/<scenario-name>/".into(),
        ],
    }])
}

fn check_fixture_testing_docs_inner(fixture_readme_path: &Path) -> Result<()> {
    check_docs_contain([DocExpectation {
        path: fixture_readme_path.into(),
        required_phrases: vec![
            "Snapshot Versus Direct Assertions".into(),
            "Use snapshots when".into(),
            "Use direct assertions when".into(),
            "Prefer direct assertions for invariants and snapshots for reviewable artifacts".into(),
        ],
    }])
}

fn markdown_has_heading_outside_code_block(source: &str, heading: &str) -> bool {
    let mut in_fenced_code_block = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_fenced_code_block = !in_fenced_code_block;
            continue;
        }

        if !in_fenced_code_block && trimmed == heading {
            return true;
        }
    }

    false
}

fn has_placeholder_marker(source: &str) -> bool {
    source.lines().any(|line| {
        starts_public_keyword(line.trim_start(), "pub struct") && line.contains("Placeholder")
    })
}

fn has_non_placeholder_public_item(source: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        (starts_public_keyword(line, "pub enum")
            || starts_public_keyword(line, "pub fn")
            || starts_public_keyword(line, "pub trait")
            || starts_public_keyword(line, "pub type")
            || starts_public_keyword(line, "pub const")
            || starts_public_keyword(line, "pub static"))
            || (starts_public_keyword(line, "pub struct") && !line.contains("Placeholder"))
    })
}

fn starts_public_keyword(line: &str, keyword: &str) -> bool {
    line == keyword || line.starts_with(&format!("{keyword} "))
}

fn collect_crate_sources_inner(directory: &Path, source_paths: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_crate_sources_inner(&path, source_paths)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            source_paths.push(path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::{
        DocExpectation, WorkspaceCrate, check_ci_workflow_inner, check_crate_inventory_docs_inner,
        check_demo_docs_inner, check_deny_config_inner, check_docs_contain,
        check_feedback_triage_docs_inner, check_fixture_directories_inner,
        check_fixture_naming_docs_inner, check_fixture_testing_docs_inner,
        check_future_session_docs_inner, check_issue_templates_inner,
        check_known_limitations_docs_inner, check_license_files_inner,
        check_no_secret_content_reads_inner, check_placeholder_sources,
        check_readme_roadmap_link_inner, check_release_planning_inner, check_report_language_inner,
        check_security_assumptions_docs_inner, check_security_policy_inner,
        check_toolchain_pin_inner, collect_crate_sources, collect_workspace_members,
        contains_crate_name, forbidden_report_overclaim_phrases, forbidden_secret_content_patterns,
        has_non_placeholder_public_item, has_placeholder_marker, required_ci_run_commands,
        secret_content_guard_source_roots, workflow_installs_toolchain,
    };

    const PLACEHOLDER_SOURCE: &str = "\
//! Core domain placeholders for future Kply session primitives.

/// Placeholder marker for the future core session model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorePlaceholder;
";
    const APACHE_LICENSE_SOURCE: &str = "\
Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/
";
    const NOTICE_SOURCE: &str = "\
Kply
Copyright 2026 Kubeply

This product includes software developed by Kubeply.
";
    const STRICT_DENY_CONFIG: &str = r#"
[advisories]
yanked = "deny"
ignore = []

[licenses]
allow = [
  "Apache-2.0",
  "MIT",
  "Unicode-3.0",
]
confidence-threshold = 0.8

[bans]
multiple-versions = "deny"
wildcards = "allow"
highlight = "all"
"#;
    const DIST_CONFIG: &str = r#"
[dist]
cargo-dist-version = "0.32.0"
packages = ["kply-cli"]
installers = ["shell"]
checksum = "sha256"
github-attestations = true
targets = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl", "x86_64-apple-darwin", "aarch64-apple-darwin"]
pr-run-mode = "plan"
allow-dirty = ["ci"]
"#;
    const DIST_TARGETS_LINE: &str = "targets = [\"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-gnu\", \"x86_64-unknown-linux-musl\", \"aarch64-unknown-linux-musl\", \"x86_64-apple-darwin\", \"aarch64-apple-darwin\"]";
    const RELEASE_PLAN_WORKFLOW: &str = r#"
name: release

on:
  pull_request:
  push:
    tags:
      - "**[0-9]+.[0-9]+.[0-9]+*"

jobs:
  dist-plan:
    steps:
      - name: Plan release
        run: dist plan
      - name: Build release artifacts
        run: dist build --artifacts=global
      - name: Host release artifacts
        run: dist host --steps=upload --steps=release
      - name: Create GitHub Release
        run: gh release create v0.1.0 artifacts/*
"#;
    const CI_WORKFLOW: &str = r#"
name: ci

on:
  pull_request:
  push:
    branches:
      - main
  merge_group:

permissions:
  contents: read

jobs:
  quality:
    steps:
      - name: Lint workflows
        uses: raven-actions/actionlint@205b530c5d9fa8f44ae9ed59f341a0db994aa6f8 # v2
      - name: Install Rust
        uses: dtolnay/rust-toolchain@3c5f7ea28cd621ae0bf5283f0e981fb97b8a7af9 # master
      - name: Check formatting
        run: cargo fmt --all -- --check
      - name: Check all targets
        run: cargo check --all-targets --all-features --locked
      - name: Lint
        run: cargo clippy --all-targets --all-features --locked -- -D warnings
      - name: Test
        run: cargo test --all-targets --all-features --locked
      - name: Test fixture helpers
        run: cargo test -p kply-test --locked
      - name: Check CI workflow
        run: cargo xtask check-ci-workflow
      - name: Check crate inventory docs
        run: cargo xtask check-crate-inventory-docs
      - name: Check demo docs
        run: cargo xtask check-demo-docs
      - name: Check cargo-deny config
        run: cargo xtask check-deny-config
      - name: Check fixture directories
        run: cargo xtask check-fixture-directories
      - name: Check fixture naming docs
        run: cargo xtask check-fixture-naming-docs
      - name: Check fixture testing docs
        run: cargo xtask check-fixture-testing-docs
      - name: Check feedback triage docs
        run: cargo xtask check-feedback-triage-docs
      - name: Check future session docs
        run: cargo xtask check-future-session-docs
      - name: Check issue templates
        run: cargo xtask check-issue-templates
      - name: Check known limitations docs
        run: cargo xtask check-known-limitations-docs
      - name: Check license files
        run: cargo xtask check-license-files
      - name: Check module docs
        run: cargo xtask check-module-docs
      - name: Check no Secret content reads
        run: cargo xtask check-no-secret-content-reads
      - name: Check placeholder docs
        run: cargo xtask check-placeholder-docs
      - name: Check placeholders
        run: cargo xtask check-placeholders
      - name: Check report language
        run: cargo xtask check-report-language
      - name: Check README roadmap link
        run: cargo xtask check-readme-roadmap-link
      - name: Check release planning
        run: cargo xtask check-release-planning
      - name: Check security assumptions docs
        run: cargo xtask check-security-assumptions-docs
      - name: Check security policy
        run: cargo xtask check-security-policy
      - name: Check toolchain pin
        run: cargo xtask check-toolchain-pin
      - name: Check dependencies
        uses: EmbarkStudios/cargo-deny-action@a531616d8ce3b9177443e48a1159bc945a099823 # v2
"#;

    #[test]
    fn accepts_placeholder_only_sources() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(temp.path(), "core.rs", PLACEHOLDER_SOURCE);

        check_placeholder_sources([source_path]).expect("placeholder source should be valid");
    }

    #[test]
    fn rejects_extra_public_items_with_path_in_error() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "core.rs",
            "\
//! Core domain placeholders for future Kply session primitives.

pub struct CorePlaceholder;

pub fn create_session() {}
",
        );

        let error = check_placeholder_sources([&source_path])
            .expect_err("extra public item should be rejected");
        let error = error.to_string();

        assert!(error.contains("product crate source file(s) are not placeholder-only"));
        assert!(error.contains(&source_path.display().to_string()));
    }

    #[test]
    fn rejects_sources_missing_placeholder_marker() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "core.rs",
            "\
//! Core domain placeholders for future Kply session primitives.

pub struct CoreModel;
",
        );

        let error = check_placeholder_sources([&source_path])
            .expect_err("missing placeholder marker should be rejected");

        assert!(error.to_string().contains("1 product crate source file(s)"));
    }

    #[test]
    fn detects_single_line_placeholder_marker() {
        assert!(has_placeholder_marker(PLACEHOLDER_SOURCE));
    }

    #[test]
    fn requires_placeholder_marker_on_public_struct_line() {
        let source = "\
//! Core domain placeholders for future Kply session primitives.

pub struct
    CorePlaceholder;
";

        assert!(!has_placeholder_marker(source));
    }

    #[test]
    fn ignores_scoped_visibility_items() {
        let source = "\
//! Core domain placeholders for future Kply session primitives.

pub struct CorePlaceholder;
pub(crate) struct InternalModel;
pub(super) fn helper() {}
";

        assert!(!has_non_placeholder_public_item(source));
    }

    #[test]
    fn detects_extra_public_items_without_placeholder_name() {
        let source = "\
//! Core domain placeholders for future Kply session primitives.

pub struct CorePlaceholder;
pub enum SessionState {}
";

        assert!(has_non_placeholder_public_item(source));
    }

    #[test]
    fn permits_public_placeholder_struct_only() {
        assert!(!has_non_placeholder_public_item(PLACEHOLDER_SOURCE));
    }

    #[test]
    fn detects_multiline_public_item_header_as_non_placeholder() {
        let source = "\
//! Core domain placeholders for future Kply session primitives.

pub struct CorePlaceholder;
pub fn
    create_session() {}
";

        assert!(has_non_placeholder_public_item(source));
    }

    #[test]
    fn accepts_docs_with_required_placeholder_phrases() {
        let temp = TempDir::new().expect("temp dir should be created");
        let doc_path = write_source(
            temp.path(),
            "README.md",
            "This scaffold contains placeholders only for a future Kply session.",
        );

        check_docs_contain([DocExpectation {
            path: doc_path,
            required_phrases: vec![
                "placeholders only".to_owned(),
                "future Kply session".to_owned(),
            ],
        }])
        .expect("doc should include required placeholder phrases");
    }

    #[test]
    fn rejects_docs_missing_placeholder_phrases() {
        let temp = TempDir::new().expect("temp dir should be created");
        let doc_path = write_source(temp.path(), "README.md", "This doc overclaims behavior.");

        let error = check_docs_contain([DocExpectation {
            path: doc_path,
            required_phrases: vec!["placeholders only".to_owned()],
        }])
        .expect_err("doc missing placeholder phrase should fail");

        assert!(
            error
                .to_string()
                .contains("1 placeholder documentation phrase(s) missing")
        );
    }

    #[test]
    fn accepts_future_session_docs_with_current_status_notes() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "\
Implementation in progress.
The workspace now includes early runtime check support.
Session mutation commands require explicit `--apply` confirmation.
",
        );
        let architecture_path = write_nested_source(
            temp.path(),
            "docs/architecture.md",
            "\
Real session planning and Kubernetes execution are now implemented.
Gateway API routing groundwork has started.
",
        );
        let product_path = write_nested_source(
            temp.path(),
            "docs/product.md",
            "\
This is a roadmap hypothesis, partially implemented behavior.
runtime checks are landing.
Gateway API routing groundwork has started.
",
        );

        check_future_session_docs_inner([readme_path, architecture_path, product_path])
            .expect("current session docs with notes should pass");
    }

    #[test]
    fn rejects_future_session_docs_missing_current_status_note() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "\
Implementation in progress.
The workspace now includes early runtime check support.
Session mutation commands require explicit `--apply` confirmation.
",
        );
        let architecture_path =
            write_nested_source(temp.path(), "docs/architecture.md", "Future sessions.\n");
        let product_path = write_nested_source(
            temp.path(),
            "docs/product.md",
            "\
This is a roadmap hypothesis, partially implemented behavior.
runtime checks are landing.
Gateway API routing groundwork has started.
",
        );

        let error = check_future_session_docs_inner([readme_path, architecture_path, product_path])
            .expect_err("future session docs missing current status should fail");

        assert!(error.to_string().contains("placeholder documentation"));
    }

    #[test]
    fn accepts_feedback_issue_templates() {
        let temp = TempDir::new().expect("temp dir should be created");
        let config_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/config.yml",
            "blank_issues_enabled: false\nSecurity reports\n/security/policy\n",
        );
        let routing_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/routing-environment.yml",
            "\
name: Routing environment
labels:
  - routing
Gateway API implementation, ingress controller, service mesh
missing route adapter request
Adapter gap
Do not include Secret values
Route strategy
Sanitized Kply output
",
        );
        let discovery_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/kubernetes-discovery-bug.yml",
            "\
name: Kubernetes discovery bug
discovery
Do not include Secret values
Workload shape
Secret metadata references
Expected discovery result
",
        );
        let planning_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/session-planning-gap.yml",
            "\
name: Session planning gap
session-planning
Do not include Secret values
Command and sanitized config
unsupported feature warnings and risk notes
repeated policy need
Policy gap
Safety constraints
",
        );
        let agent_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/agent-workflow-request.yml",
            "\
name: Agent workflow request
agent-workflow
Codex, Claude Code, Cursor
Do not include Secret values
Failure mode to prevent
Useful Kply behavior
",
        );
        let roadmap_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/roadmap-request.yml",
            "\
name: Roadmap request
roadmap
real user feedback, demos, benchmarks
Do not include Secret values
Evidence type
infra-bench failure
repeated policy need
Requested roadmap change
Affected agent-infra workflow
Success criteria
",
        );

        check_issue_templates_inner([
            config_path,
            routing_path,
            discovery_path,
            planning_path,
            agent_path,
            roadmap_path,
        ])
        .expect("feedback issue templates should pass");
    }

    #[test]
    fn rejects_feedback_issue_templates_without_secret_warning() {
        let temp = TempDir::new().expect("temp dir should be created");
        let config_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/config.yml",
            "blank_issues_enabled: false\nSecurity reports\n/security/policy\n",
        );
        let routing_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/routing-environment.yml",
            "\
name: Routing environment
labels:
  - routing
Gateway API implementation, ingress controller, service mesh
missing route adapter request
Adapter gap
Route strategy
Sanitized Kply output
",
        );
        let discovery_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/kubernetes-discovery-bug.yml",
            "\
name: Kubernetes discovery bug
discovery
Do not include Secret values
Workload shape
Secret metadata references
Expected discovery result
",
        );
        let planning_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/session-planning-gap.yml",
            "\
name: Session planning gap
session-planning
Do not include Secret values
Command and sanitized config
unsupported feature warnings and risk notes
repeated policy need
Policy gap
Safety constraints
",
        );
        let agent_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/agent-workflow-request.yml",
            "\
name: Agent workflow request
agent-workflow
Codex, Claude Code, Cursor
Do not include Secret values
Failure mode to prevent
Useful Kply behavior
",
        );
        let roadmap_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/roadmap-request.yml",
            "\
name: Roadmap request
roadmap
real user feedback, demos, benchmarks
Do not include Secret values
Evidence type
infra-bench failure
repeated policy need
Requested roadmap change
Affected agent-infra workflow
Success criteria
",
        );

        let error = check_issue_templates_inner([
            config_path,
            routing_path,
            discovery_path,
            planning_path,
            agent_path,
            roadmap_path,
        ])
        .expect_err("feedback templates without routing Secret warning should fail");

        assert!(error.to_string().contains("documentation phrase"));
    }

    #[test]
    fn accepts_feedback_triage_docs() {
        let temp = TempDir::new().expect("temp dir should be created");
        let feedback_triage_path = write_nested_source(
            temp.path(),
            "docs/feedback-triage.md",
            "\
# Feedback Triage
## Missing Route Adapters
## Policy Needs
missing route adapter
Three separate users or organizations
repeated route adapter request
repeated policy need
same policy boundary
minimum deterministic decision
Do not include Secret values
explicit permission exists
OpenSpec change
",
        );

        check_feedback_triage_docs_inner(feedback_triage_path)
            .expect("feedback triage docs should pass");
    }

    #[test]
    fn rejects_feedback_triage_docs_without_repetition_threshold() {
        let temp = TempDir::new().expect("temp dir should be created");
        let feedback_triage_path = write_nested_source(
            temp.path(),
            "docs/feedback-triage.md",
            "\
# Feedback Triage
## Missing Route Adapters
## Policy Needs
missing route adapter
repeated route adapter request
repeated policy need
same policy boundary
minimum deterministic decision
Do not include Secret values
explicit permission exists
OpenSpec change
",
        );

        let error = check_feedback_triage_docs_inner(feedback_triage_path)
            .expect_err("feedback triage docs without repetition threshold should fail");

        assert!(error.to_string().contains("documentation phrase"));
    }

    #[test]
    fn accepts_first_release_known_limitations_docs() {
        let temp = TempDir::new().expect("temp dir should be created");
        let first_release_path = write_nested_source(
            temp.path(),
            "docs/first-release.md",
            "\
# First Release Scope

## Known Limitations

Kply `v0.1.0` is an evaluation release, not a production safety guarantee.

- No automatic promotion is included.
- `kply route apply` is a no-op placeholder.
- Experimental live apply can create sandbox Deployment and Service resources.
- Preview Service and no-route checks do not prove edge routing behavior.
- Kply never reads Kubernetes Secret values.
- No hosted policy, team approval, audit retention, or reporting service is included.
- No long-running in-cluster controller reconciles sessions after the CLI exits.
- Runtime checks report evidence, not approval to deploy.
- The JSON contracts are stable only for `v0.1.0` evaluation.
- The local demo is bounded to the Kind ecommerce fixture.
",
        );

        check_known_limitations_docs_inner(first_release_path)
            .expect("known limitations docs should pass");
    }

    #[test]
    fn rejects_first_release_known_limitations_docs_without_runtime_evidence_boundary() {
        let temp = TempDir::new().expect("temp dir should be created");
        let first_release_path = write_nested_source(
            temp.path(),
            "docs/first-release.md",
            "\
# First Release Scope

## Known Limitations

Kply `v0.1.0` is an evaluation release, not a production safety guarantee.

- No automatic promotion is included.
- `kply route apply` is a no-op placeholder.
- Experimental live apply can create sandbox Deployment and Service resources.
- Preview Service and no-route checks do not prove edge routing behavior.
- Kply never reads Kubernetes Secret values.
- No hosted policy, team approval, audit retention, or reporting service is included.
- No long-running in-cluster controller reconciles sessions after the CLI exits.
- The JSON contracts are stable only for `v0.1.0` evaluation.
- The local demo is bounded to the Kind ecommerce fixture.
",
        );

        let error = check_known_limitations_docs_inner(first_release_path)
            .expect_err("known limitations docs missing approval boundary should fail");

        assert!(error.to_string().contains("documentation phrase"));
    }

    #[test]
    fn accepts_first_release_security_assumptions_docs() {
        let temp = TempDir::new().expect("temp dir should be created");
        let first_release_path = write_nested_source(
            temp.path(),
            "docs/first-release.md",
            "\
# First Release Scope

## Security Assumptions

Kply `v0.1.0` assumes the caller already has Kubernetes credentials.

- Kply does not bypass Kubernetes RBAC.
- Use a dedicated service account.
- Run Kply against a namespace the platform owner has approved.
- Admission policy should enforce ownership labels.
- Do not pass production admin kubeconfigs to agents.
- Network isolation, egress policy, and database permissions remain platform responsibilities.
- Kply output is local CLI evidence and must be retained by the caller.
- Supply-chain trust depends on GitHub Release artifacts, SHA-256 checksums, and attestations.
- Secret values remain out of scope.
",
        );

        check_security_assumptions_docs_inner(first_release_path)
            .expect("security assumptions docs should pass");
    }

    #[test]
    fn rejects_first_release_security_assumptions_docs_without_rbac_boundary() {
        let temp = TempDir::new().expect("temp dir should be created");
        let first_release_path = write_nested_source(
            temp.path(),
            "docs/first-release.md",
            "\
# First Release Scope

## Security Assumptions

Kply `v0.1.0` assumes the caller already has Kubernetes credentials.

- Use a dedicated service account.
- Run Kply against a namespace the platform owner has approved.
- Admission policy should enforce ownership labels.
- Do not pass production admin kubeconfigs to agents.
- Network isolation, egress policy, and database permissions remain platform responsibilities.
- Kply output is local CLI evidence and must be retained by the caller.
- Supply-chain trust depends on GitHub Release artifacts, SHA-256 checksums, and attestations.
- Secret values remain out of scope.
",
        );

        let error = check_security_assumptions_docs_inner(first_release_path)
            .expect_err("security assumptions docs missing RBAC boundary should fail");

        assert!(error.to_string().contains("documentation phrase"));
    }

    #[test]
    fn accepts_security_policy_docs() {
        let temp = TempDir::new().expect("temp dir should be created");
        let security_path = write_source(
            temp.path(),
            "SECURITY.md",
            "\
# Security Policy

## Supported Versions

Security reports are accepted for the latest released `v0.1.x` version.

Use GitHub private vulnerability reporting.
Do not open a public issue for a vulnerability.

Kply reads or prints Kubernetes Secret values.
Kply mutates resources without an explicit apply or confirmation boundary.
Release artifacts, checksums, installers, or attestations.
There is no paid bug bounty program.
Do not send live credentials, Secret values, production kubeconfigs.
",
        );
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "## Security\n\nSee [SECURITY.md](SECURITY.md) for private vulnerability reporting.\n",
        );
        let issue_config_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/config.yml",
            "\
Security reports
https://github.com/kubeply/kply/security/policy
private security advisory flow
",
        );

        check_security_policy_inner([security_path, readme_path, issue_config_path])
            .expect("security policy docs should pass");
    }

    #[test]
    fn rejects_security_policy_without_private_reporting_boundary() {
        let temp = TempDir::new().expect("temp dir should be created");
        let security_path = write_source(
            temp.path(),
            "SECURITY.md",
            "\
# Security Policy

## Supported Versions

Security reports are accepted for the latest released `v0.1.x` version.

Kply reads or prints Kubernetes Secret values.
Kply mutates resources without an explicit apply or confirmation boundary.
Release artifacts, checksums, installers, or attestations.
There is no paid bug bounty program.
Do not send live credentials, Secret values, production kubeconfigs.
",
        );
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "## Security\n\nSee [SECURITY.md](SECURITY.md) for private vulnerability reporting.\n",
        );
        let issue_config_path = write_nested_source(
            temp.path(),
            ".github/ISSUE_TEMPLATE/config.yml",
            "\
Security reports
https://github.com/kubeply/kply/security/policy
private security advisory flow
",
        );

        let error = check_security_policy_inner([security_path, readme_path, issue_config_path])
            .expect_err("security policy without private reporting should fail");

        assert!(error.to_string().contains("documentation phrase"));
    }

    #[test]
    fn accepts_local_demo_docs_with_bounded_agent_workflow() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "\
## Local Demo

See [docs/demo-kind.md](docs/demo-kind.md) for the current manual Kind setup guide.
",
        );
        let kind_path = write_nested_source(
            temp.path(),
            "docs/demo-kind.md",
            "\
# Local Kind Demo

See demo-agent.md and scripts/demo-walkthrough.sh.

Run kply demo doctor, kply demo install, kply demo reset, and kply demo teardown.
Run kply session create, kply check run, and kply session cleanup.
Create the cluster with kind create cluster --name kply-demo.
Use fixtures/demo/ecommerce-basic/kply.yaml in the kply-demo namespace.
",
        );
        let agent_path = write_nested_source(
            temp.path(),
            "docs/demo-agent.md",
            "\
# Coding Agent Demo Guide

Use Kubernetes context kind-kply-demo only.
Do not touch resources outside the kply-demo namespace.
Do not read Kubernetes Secret values.
Use kply session create --apply, kply check run, and kply session cleanup --apply.
Use fixtures/demo/ecommerce-basic/manifests/ and scripts/demo-walkthrough.sh.
Current route apply output includes status: \"not_implemented\".
It does not yet prove live sandbox routing.
",
        );
        let fixture_path = write_nested_source(
            temp.path(),
            "fixtures/demo/ecommerce-basic/README.md",
            "\
See docs/demo-kind.md, docs/demo-agent.md, and scripts/demo-walkthrough.sh.
Backend variants include backend-broken.yaml and backend-fixed.yaml.
",
        );

        check_demo_docs_inner([readme_path, kind_path, agent_path, fixture_path])
            .expect("local demo docs should pass");
    }

    #[test]
    fn rejects_local_demo_docs_without_agent_boundary() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "## Local Demo\nSee [docs/demo-kind.md](docs/demo-kind.md) for the current manual Kind setup guide.\n",
        );
        let kind_path = write_nested_source(
            temp.path(),
            "docs/demo-kind.md",
            "\
# Local Kind Demo
demo-agent.md
scripts/demo-walkthrough.sh
kply demo doctor
kply demo install
kply demo reset
kply demo teardown
kind create cluster --name kply-demo
fixtures/demo/ecommerce-basic/kply.yaml
kply-demo
",
        );
        let agent_path = write_nested_source(
            temp.path(),
            "docs/demo-agent.md",
            "\
# Coding Agent Demo Guide
Use Kubernetes context kind-kply-demo only.
",
        );
        let fixture_path = write_nested_source(
            temp.path(),
            "fixtures/demo/ecommerce-basic/README.md",
            "\
docs/demo-kind.md
docs/demo-agent.md
scripts/demo-walkthrough.sh
backend-broken.yaml
backend-fixed.yaml
",
        );

        let error = check_demo_docs_inner([readme_path, kind_path, agent_path, fixture_path])
            .expect_err("demo docs without agent boundary should fail");

        assert!(error.to_string().contains("documentation phrase"));
    }

    #[test]
    fn accepts_secret_metadata_without_content_reads() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "k8s.rs",
            "\
//! Kubernetes adapters.

pub struct IngressTlsSummary {
    pub secret_name: Option<String>,
}
",
        );

        check_no_secret_content_reads_inner([&source_path], forbidden_secret_content_patterns())
            .expect("secret metadata references should pass");
    }

    #[test]
    fn configures_secret_content_guard_source_roots_for_product_crates() {
        let roots = secret_content_guard_source_roots();

        assert!(roots.contains(&"crates/kply-checks/src"));
        assert!(roots.contains(&"crates/kply-cli/src"));
        assert!(roots.contains(&"crates/kply-config/src"));
        assert!(roots.contains(&"crates/kply-core/src"));
        assert!(roots.contains(&"crates/kply-k8s/src"));
        assert!(roots.contains(&"crates/kply-routing/src"));
        assert!(!roots.contains(&"crates/xtask/src"));

        let unique_roots = roots.iter().collect::<BTreeSet<_>>();
        assert_eq!(
            unique_roots.len(),
            roots.len(),
            "secret content guard roots should not contain duplicates"
        );

        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask manifest should live under crates/xtask");
        let mut source_paths = Vec::new();
        for root in roots {
            source_paths.extend(
                collect_crate_sources(workspace_root.join(root))
                    .expect("product source root should be readable"),
            );
        }
        source_paths.sort();
        source_paths.dedup();

        assert!(
            source_paths
                .iter()
                .any(|path| path.ends_with("crates/kply-cli/src/main.rs"))
        );
        assert!(
            source_paths
                .iter()
                .any(|path| path.ends_with("crates/kply-core/src/lib.rs"))
        );
        assert!(
            source_paths
                .iter()
                .all(|path| !path.ends_with("crates/xtask/src/main.rs"))
        );
        let unique_source_paths = source_paths.iter().collect::<BTreeSet<_>>();
        assert_eq!(
            unique_source_paths.len(),
            source_paths.len(),
            "secret content guard aggregation should not contain duplicate source paths"
        );
    }

    #[test]
    fn rejects_secret_type_imports_and_content_fields() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "k8s.rs",
            "\
//! Kubernetes adapters.

use k8s_openapi::api::core::v1::Secret;

fn read_secret(secret: Secret) {
    let _ = secret.data;
}
",
        );

        let error = check_no_secret_content_reads_inner(
            [&source_path],
            forbidden_secret_content_patterns(),
        )
        .expect_err("secret content reads should fail");

        assert!(
            error
                .to_string()
                .contains("forbidden Secret content access pattern")
        );
    }

    #[test]
    fn rejects_typed_secret_content_field_access_with_short_identifier() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "k8s.rs",
            "\
//! Kubernetes adapters.

fn read_secret(s: Secret) {
    let _ = s.string_data;
}
",
        );

        let error = check_no_secret_content_reads_inner(
            [&source_path],
            forbidden_secret_content_patterns(),
        )
        .expect_err("typed Secret content reads should fail");

        assert!(
            error
                .to_string()
                .contains("forbidden Secret content access pattern")
        );
    }

    #[test]
    fn rejects_lifetime_secret_reference_content_field_access() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "k8s.rs",
            "\
//! Kubernetes adapters.

fn read_secret<'a>(s: &'a Secret) {
    let _ = s.data;
}
",
        );

        let error = check_no_secret_content_reads_inner(
            [&source_path],
            forbidden_secret_content_patterns(),
        )
        .expect_err("lifetime Secret references should still be tracked");

        assert!(
            error
                .to_string()
                .contains("forbidden Secret content access pattern")
        );
    }

    #[test]
    fn ignores_secret_content_patterns_in_comments_and_strings() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "k8s.rs",
            "\
//! Kubernetes adapters.

fn describe() {
    // secret.data should stay documented without failing.
    let message = \"secret.string_data is forbidden\";
}
",
        );

        check_no_secret_content_reads_inner([&source_path], forbidden_secret_content_patterns())
            .expect("comments and strings should be ignored");
    }

    #[test]
    fn ignores_char_literals_before_secret_content_checks() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "k8s.rs",
            "\
//! Kubernetes adapters.

fn describe() {
    let quote = '\\'';
    let newline = '\\n';
    let plain = 'x';
    let metadata = dynamic.data;
}
",
        );

        check_no_secret_content_reads_inner([&source_path], forbidden_secret_content_patterns())
            .expect("char literals should not corrupt later scanning");
    }

    #[test]
    fn ignores_block_comments_and_raw_strings() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "k8s.rs",
            "\
//! Kubernetes adapters.

/*
let _ = secret.data;
*/

fn describe() {
    let raw = r#\"
secret.string_data
\"#;
    let metadata = dynamic.data;
}
",
        );

        check_no_secret_content_reads_inner([&source_path], forbidden_secret_content_patterns())
            .expect("block comments and raw strings should be ignored");
    }

    #[test]
    fn accepts_conservative_report_language() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "report.md",
            "\
Kply did not report a blocking condition in the available evidence.
A human still needs to review and promote outside Kply.
",
        );

        check_report_language_inner([&source_path], forbidden_report_overclaim_phrases())
            .expect("conservative report language should pass");
    }

    #[test]
    fn rejects_report_overclaim_language() {
        let temp = TempDir::new().expect("temp dir should be created");
        let source_path = write_source(
            temp.path(),
            "report.md",
            "This change is safe to deploy after the Kply session.\n",
        );

        let error =
            check_report_language_inner([&source_path], forbidden_report_overclaim_phrases())
                .expect_err("deployment approval language should fail");

        assert!(error.to_string().contains("report language overclaim"));
    }

    #[test]
    fn accepts_required_fixture_directories() {
        let temp = TempDir::new().expect("temp dir should be created");
        let fixtures_root = temp.path().join("fixtures");

        for directory in ["cli", "config", "manifests"] {
            fs::create_dir_all(fixtures_root.join(directory))
                .expect("fixture directory should be created");
        }

        check_fixture_directories_inner(&fixtures_root, &["cli", "config", "manifests"])
            .expect("required fixture directories should pass");
    }

    #[test]
    fn rejects_missing_fixture_directories() {
        let temp = TempDir::new().expect("temp dir should be created");
        let fixtures_root = temp.path().join("fixtures");
        fs::create_dir_all(fixtures_root.join("cli")).expect("fixture directory should be created");

        let error = check_fixture_directories_inner(&fixtures_root, &["cli", "config"])
            .expect_err("missing fixture directories should fail");

        assert!(error.to_string().contains("fixture directories missing"));
    }

    #[test]
    fn accepts_fixture_naming_docs() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "cli/<behavior-name>/\nconfig/<case-name>/kply.yaml\nmanifests/<workload-shape>/\nk8s-responses/<api-shape>/\nreports/<workflow-name>/\ndemo/<scenario-name>/\n",
        );

        check_fixture_naming_docs_inner(&readme_path)
            .expect("fixture naming docs with required patterns should pass");
    }

    #[test]
    fn rejects_fixture_naming_docs_missing_patterns() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(temp.path(), "README.md", "cli/<behavior-name>/\n");

        let error = check_fixture_naming_docs_inner(&readme_path)
            .expect_err("fixture naming docs missing patterns should fail");

        assert!(error.to_string().contains("placeholder documentation"));
    }

    #[test]
    fn accepts_fixture_testing_docs() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "## Snapshot Versus Direct Assertions\n\nUse snapshots when output is reviewable.\nUse direct assertions when behavior is small.\nPrefer direct assertions for invariants and snapshots for reviewable artifacts.\n",
        );

        check_fixture_testing_docs_inner(&readme_path)
            .expect("fixture testing docs with required guidance should pass");
    }

    #[test]
    fn rejects_fixture_testing_docs_missing_guidance() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(temp.path(), "README.md", "## Fixtures\n");

        let error = check_fixture_testing_docs_inner(&readme_path)
            .expect_err("fixture testing docs missing guidance should fail");

        assert!(error.to_string().contains("placeholder documentation"));
    }

    #[test]
    fn accepts_readme_with_roadmap_link() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "# Kply\n\n## Roadmap\n\nSee [docs/implementation-roadmap.md](docs/implementation-roadmap.md).\n",
        );

        check_readme_roadmap_link_inner(&readme_path).expect("README roadmap link should pass");
    }

    #[test]
    fn rejects_readme_without_roadmap_link() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(temp.path(), "README.md", "# Kply\n\n## Development\n");

        let error = check_readme_roadmap_link_inner(&readme_path)
            .expect_err("README without roadmap link should fail");

        assert!(error.to_string().contains("Roadmap section"));
    }

    #[test]
    fn rejects_readme_with_heading_but_no_link() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(temp.path(), "README.md", "# Kply\n\n## Roadmap\n");

        let error = check_readme_roadmap_link_inner(&readme_path)
            .expect_err("README without roadmap link should fail");

        assert!(error.to_string().contains("Roadmap section"));
    }

    #[test]
    fn rejects_readme_with_link_but_no_heading() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "# Kply\n\nSee [docs/implementation-roadmap.md](docs/implementation-roadmap.md).\n",
        );

        let error = check_readme_roadmap_link_inner(&readme_path)
            .expect_err("README without roadmap heading should fail");

        assert!(error.to_string().contains("Roadmap section"));
    }

    #[test]
    fn rejects_readme_with_wrong_roadmap_heading_level() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "# Kply\n\n### Roadmap\n\nSee [docs/implementation-roadmap.md](docs/implementation-roadmap.md).\n",
        );

        let error = check_readme_roadmap_link_inner(&readme_path)
            .expect_err("README with wrong roadmap heading level should fail");

        assert!(error.to_string().contains("Roadmap section"));
    }

    #[test]
    fn rejects_readme_with_concatenated_roadmap_heading() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "# Kply\n\n## RoadmapPlanning\n\nSee [docs/implementation-roadmap.md](docs/implementation-roadmap.md).\n",
        );

        let error = check_readme_roadmap_link_inner(&readme_path)
            .expect_err("README with concatenated roadmap heading should fail");

        assert!(error.to_string().contains("Roadmap section"));
    }

    #[test]
    fn rejects_readme_with_roadmap_heading_in_code_block() {
        let temp = TempDir::new().expect("temp dir should be created");
        let readme_path = write_source(
            temp.path(),
            "README.md",
            "# Kply\n\n```md\n## Roadmap\n```\n\nSee [docs/implementation-roadmap.md](docs/implementation-roadmap.md).\n",
        );

        let error = check_readme_roadmap_link_inner(&readme_path)
            .expect_err("README with roadmap heading in code block should fail");

        assert!(error.to_string().contains("Roadmap section"));
    }

    #[test]
    fn collects_workspace_members_from_manifest() {
        let manifest = r#"
[workspace]
members = [
    "crates/kply-cli",
    "crates/xtask",
]
resolver = "3"
"#;

        assert_eq!(
            collect_workspace_members(manifest).expect("workspace members should parse"),
            vec!["crates/kply-cli", "crates/xtask"]
        );
    }

    #[test]
    fn collects_inline_workspace_members_from_manifest() {
        let manifest = r#"
[workspace]
members = ["crates/kply-cli", "crates/xtask"]
"#;

        assert_eq!(
            collect_workspace_members(manifest).expect("workspace members should parse"),
            vec!["crates/kply-cli", "crates/xtask"]
        );
    }

    #[test]
    fn matches_crate_names_with_boundaries() {
        assert!(contains_crate_name(
            "`kply-core`: domain model",
            "kply-core"
        ));
        assert!(!contains_crate_name(
            "`kply-core-extra`: separate crate",
            "kply-core"
        ));
        assert!(!contains_crate_name(
            "`my-kply-core`: separate crate",
            "kply-core"
        ));
    }

    #[test]
    fn accepts_docs_with_complete_crate_inventory() {
        let temp = TempDir::new().expect("temp dir should be created");
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace]
members = [
    "crates/kply-cli",
    "crates/xtask",
]
"#,
        );
        let agents_path = write_source(temp.path(), "AGENTS.md", "kply-cli\nxtask\n");
        let contributing_path = write_source(temp.path(), "CONTRIBUTING.md", "kply-cli\nxtask\n");
        let crates_path = write_source(temp.path(), "crates.md", "kply-cli\nxtask\n");

        check_crate_inventory_docs_inner(
            &manifest_path,
            [&agents_path, &contributing_path, &crates_path],
            test_workspace_crates(),
        )
        .expect("complete crate inventory docs should pass");
    }

    #[test]
    fn accepts_manifest_members_in_different_order() {
        let temp = TempDir::new().expect("temp dir should be created");
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace]
members = [
    "crates/xtask",
    "crates/kply-cli",
]
"#,
        );
        let agents_path = write_source(temp.path(), "AGENTS.md", "kply-cli\nxtask\n");

        check_crate_inventory_docs_inner(&manifest_path, [&agents_path], test_workspace_crates())
            .expect("manifest member order should not matter");
    }

    #[test]
    fn rejects_docs_missing_crate_inventory_entries() {
        let temp = TempDir::new().expect("temp dir should be created");
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace]
members = [
    "crates/kply-cli",
    "crates/xtask",
]
"#,
        );
        let agents_path = write_source(temp.path(), "AGENTS.md", "kply-cli\n");

        let error = check_crate_inventory_docs_inner(
            &manifest_path,
            [&agents_path],
            test_workspace_crates(),
        )
        .expect_err("missing crate inventory entry should fail");

        assert!(
            error
                .to_string()
                .contains("crate inventory entries missing")
        );
    }

    #[test]
    fn rejects_manifest_inventory_mismatches() {
        let temp = TempDir::new().expect("temp dir should be created");
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace]
members = [
    "crates/kply-cli",
]
"#,
        );
        let agents_path = write_source(temp.path(), "AGENTS.md", "kply-cli\nxtask\n");

        let error = check_crate_inventory_docs_inner(
            &manifest_path,
            [&agents_path],
            test_workspace_crates(),
        )
        .expect_err("manifest inventory mismatch should fail");

        assert!(
            error
                .to_string()
                .contains("does not match Cargo.toml members")
        );
    }

    #[test]
    fn accepts_apache_license_files_and_workspace_license_inheritance() {
        let temp = TempDir::new().expect("temp dir should be created");
        let license_path = write_source(temp.path(), "LICENSE", APACHE_LICENSE_SOURCE);
        let notice_path = write_source(temp.path(), "NOTICE", NOTICE_SOURCE);
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace.package]
license = "Apache-2.0"
"#,
        );
        write_crate_manifests(temp.path(), "license.workspace = true");

        check_license_files_inner(
            &license_path,
            &notice_path,
            &manifest_path,
            test_workspace_crates(),
        )
        .expect("Apache-2.0 license files should pass");
    }

    #[test]
    fn rejects_missing_apache_license_phrase() {
        let temp = TempDir::new().expect("temp dir should be created");
        let license_path = write_source(temp.path(), "LICENSE", "Apache License\n");
        let notice_path = write_source(temp.path(), "NOTICE", NOTICE_SOURCE);
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace.package]
license = "Apache-2.0"
"#,
        );
        write_crate_manifests(temp.path(), "license.workspace = true");

        let error = check_license_files_inner(
            &license_path,
            &notice_path,
            &manifest_path,
            test_workspace_crates(),
        )
        .expect_err("missing Apache phrase should fail");

        assert!(error.to_string().contains("license file issue(s) found"));
    }

    #[test]
    fn rejects_missing_notice_phrase() {
        let temp = TempDir::new().expect("temp dir should be created");
        let license_path = write_source(temp.path(), "LICENSE", APACHE_LICENSE_SOURCE);
        let notice_path = write_source(temp.path(), "NOTICE", "Kply\n");
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace.package]
license = "Apache-2.0"
"#,
        );
        write_crate_manifests(temp.path(), "license.workspace = true");

        let error = check_license_files_inner(
            &license_path,
            &notice_path,
            &manifest_path,
            test_workspace_crates(),
        )
        .expect_err("missing notice phrase should fail");

        assert!(error.to_string().contains("license file issue(s) found"));
    }

    #[test]
    fn rejects_workspace_manifest_without_apache_license() {
        let temp = TempDir::new().expect("temp dir should be created");
        let license_path = write_source(temp.path(), "LICENSE", APACHE_LICENSE_SOURCE);
        let notice_path = write_source(temp.path(), "NOTICE", NOTICE_SOURCE);
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace.package]
license = "MIT"
"#,
        );
        write_crate_manifests(temp.path(), "license.workspace = true");

        let error = check_license_files_inner(
            &license_path,
            &notice_path,
            &manifest_path,
            test_workspace_crates(),
        )
        .expect_err("non-Apache workspace license should fail");

        assert!(error.to_string().contains("license file issue(s) found"));
    }

    #[test]
    fn rejects_crate_manifest_without_workspace_license_inheritance() {
        let temp = TempDir::new().expect("temp dir should be created");
        let license_path = write_source(temp.path(), "LICENSE", APACHE_LICENSE_SOURCE);
        let notice_path = write_source(temp.path(), "NOTICE", NOTICE_SOURCE);
        let manifest_path = write_source(
            temp.path(),
            "Cargo.toml",
            r#"
[workspace.package]
license = "Apache-2.0"
"#,
        );
        write_nested_source(
            temp.path(),
            "crates/kply-cli/Cargo.toml",
            "[package]\nname = \"kply-cli\"\n",
        );
        write_nested_source(
            temp.path(),
            "crates/xtask/Cargo.toml",
            "[package]\nname = \"xtask\"\nlicense.workspace = true\n",
        );

        let error = check_license_files_inner(
            &license_path,
            &notice_path,
            &manifest_path,
            test_workspace_crates(),
        )
        .expect_err("crate manifest without workspace license should fail");

        assert!(error.to_string().contains("license file issue(s) found"));
    }

    #[test]
    fn accepts_strict_cargo_deny_config() {
        let temp = TempDir::new().expect("temp dir should be created");
        let deny_path = write_source(temp.path(), "deny.toml", STRICT_DENY_CONFIG);

        check_deny_config_inner(&deny_path).expect("strict cargo-deny config should pass");
    }

    #[test]
    fn rejects_cargo_deny_warning_for_duplicate_versions() {
        let temp = TempDir::new().expect("temp dir should be created");
        let deny_path = write_source(
            temp.path(),
            "deny.toml",
            &STRICT_DENY_CONFIG.replace(
                "multiple-versions = \"deny\"",
                "multiple-versions = \"warn\"",
            ),
        );

        let error = check_deny_config_inner(&deny_path)
            .expect_err("duplicate version warnings should fail");

        assert!(
            error
                .to_string()
                .contains("cargo-deny config issue(s) found")
        );
    }

    #[test]
    fn rejects_cargo_deny_license_allowlist_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let deny_path = write_source(
            temp.path(),
            "deny.toml",
            &STRICT_DENY_CONFIG.replace("\"MIT\",", "\"MIT\",\n  \"BSD-3-Clause\","),
        );

        let error =
            check_deny_config_inner(&deny_path).expect_err("extra allowed license should fail");

        assert!(
            error
                .to_string()
                .contains("cargo-deny config issue(s) found")
        );
    }

    #[test]
    fn rejects_cargo_deny_advisory_ignores() {
        let temp = TempDir::new().expect("temp dir should be created");
        let deny_path = write_source(
            temp.path(),
            "deny.toml",
            &STRICT_DENY_CONFIG.replace("ignore = []", "ignore = [\"RUSTSEC-0000-0000\"]"),
        );

        let error = check_deny_config_inner(&deny_path)
            .expect_err("advisory ignores should fail until justified");

        assert!(
            error
                .to_string()
                .contains("cargo-deny config issue(s) found")
        );
    }

    #[test]
    fn accepts_first_release_ci_workflow() {
        let temp = TempDir::new().expect("temp dir should be created");
        let workflow_path =
            write_nested_source(temp.path(), ".github/workflows/ci.yml", CI_WORKFLOW);

        check_ci_workflow_inner(&workflow_path).expect("first-release CI workflow should pass");
    }

    #[test]
    fn requires_every_ci_release_gate_command() {
        let temp = TempDir::new().expect("temp dir should be created");

        for (index, required_command) in required_ci_run_commands().iter().enumerate() {
            let workflow_filename =
                format!(".github/workflows/missing-required-command-{index}.yml");
            let workflow_path = write_nested_source(
                temp.path(),
                &workflow_filename,
                &CI_WORKFLOW.replace(required_command, "cargo --version"),
            );

            let error = check_ci_workflow_inner(&workflow_path)
                .expect_err("workflow missing required command should fail");

            assert!(
                error.to_string().contains(*required_command),
                "error should mention missing command {required_command}"
            );
        }
    }

    #[test]
    fn rejects_ci_workflow_with_echoed_required_command() {
        let temp = TempDir::new().expect("temp dir should be created");
        let required_command = "cargo fmt --all -- --check";
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            &CI_WORKFLOW.replace(
                &format!("run: {required_command}"),
                &format!("run: echo {required_command}"),
            ),
        );

        let error = check_ci_workflow_inner(&workflow_path)
            .expect_err("echoed required command should not satisfy CI guard");

        assert!(error.to_string().contains(required_command));
    }

    #[test]
    fn rejects_ci_workflow_without_pull_request_trigger() {
        let temp = TempDir::new().expect("temp dir should be created");
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            &CI_WORKFLOW.replace("  pull_request:\n", ""),
        );

        let error = check_ci_workflow_inner(&workflow_path)
            .expect_err("CI workflow without pull_request should fail");

        assert!(error.to_string().contains("pull_request"));
    }

    #[test]
    fn rejects_ci_workflow_without_merge_group_trigger() {
        let temp = TempDir::new().expect("temp dir should be created");
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            &CI_WORKFLOW.replace("  merge_group:\n", ""),
        );

        let error = check_ci_workflow_inner(&workflow_path)
            .expect_err("CI workflow without merge_group should fail");

        assert!(error.to_string().contains("merge_group"));
    }

    #[test]
    fn rejects_ci_workflow_without_main_push_trigger() {
        let temp = TempDir::new().expect("temp dir should be created");
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            &CI_WORKFLOW.replace("      - main", "      - release"),
        );

        let error = check_ci_workflow_inner(&workflow_path)
            .expect_err("CI workflow without main push should fail");

        assert!(error.to_string().contains("pushes to main"));
    }

    #[test]
    fn rejects_ci_workflow_without_read_only_contents_permission() {
        let temp = TempDir::new().expect("temp dir should be created");
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            &CI_WORKFLOW.replace("  contents: read", "  contents: write"),
        );

        let error = check_ci_workflow_inner(&workflow_path)
            .expect_err("CI workflow without read-only contents permission should fail");

        assert!(error.to_string().contains("contents: read"));
    }

    #[test]
    fn rejects_ci_workflow_with_job_level_write_contents_permission() {
        let temp = TempDir::new().expect("temp dir should be created");
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            &CI_WORKFLOW.replace(
                "  quality:\n    steps:",
                "  quality:\n    permissions:\n      contents: write\n    steps:",
            ),
        );

        let error = check_ci_workflow_inner(&workflow_path)
            .expect_err("CI workflow with job-level contents write should fail");

        assert!(error.to_string().contains("contents: read"));
    }

    #[test]
    fn rejects_ci_workflow_without_required_actions() {
        let temp = TempDir::new().expect("temp dir should be created");
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            &CI_WORKFLOW
                .replace("raven-actions/actionlint", "raven-actions/other")
                .replace("dtolnay/rust-toolchain", "dtolnay/other")
                .replace("EmbarkStudios/cargo-deny-action", "EmbarkStudios/other"),
        );

        let error = check_ci_workflow_inner(&workflow_path)
            .expect_err("CI workflow without required actions should fail");
        let error = error.to_string();

        assert!(error.contains("actionlint"));
        assert!(error.contains("rust-toolchain"));
        assert!(error.contains("cargo-deny-action"));
    }

    #[test]
    fn rejects_ci_workflow_with_spoofed_required_actions() {
        let temp = TempDir::new().expect("temp dir should be created");
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            &CI_WORKFLOW
                .replace(
                    "raven-actions/actionlint",
                    "example/raven-actions-actionlint",
                )
                .replace("dtolnay/rust-toolchain", "example/dtolnay-rust-toolchain")
                .replace(
                    "EmbarkStudios/cargo-deny-action",
                    "example/EmbarkStudios-cargo-deny-action",
                ),
        );

        let error = check_ci_workflow_inner(&workflow_path)
            .expect_err("spoofed action repository names should fail");
        let error = error.to_string();

        assert!(error.contains("actionlint"));
        assert!(error.contains("rust-toolchain"));
        assert!(error.contains("cargo-deny-action"));
    }

    #[test]
    fn accepts_release_packaging_workflow() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        check_release_planning_inner(&dist_path, &workflow_path)
            .expect("release packaging workflow should pass");
    }

    #[test]
    fn rejects_release_workflow_without_tag_push_trigger() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            &RELEASE_PLAN_WORKFLOW.replace(
                "  push:\n    tags:\n      - \"**[0-9]+.[0-9]+.[0-9]+*\"\n",
                "",
            ),
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release workflow without tag push should fail");

        assert!(error.to_string().contains("semver tag pushes"));
    }

    #[test]
    fn rejects_release_workflow_with_non_semver_tag_push_trigger() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            &RELEASE_PLAN_WORKFLOW.replace("**[0-9]+.[0-9]+.[0-9]+*", "release-*"),
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release workflow without semver tag push should fail");

        assert!(error.to_string().contains("semver tag pushes"));
    }

    #[test]
    fn rejects_release_workflow_without_pull_request_trigger() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            &RELEASE_PLAN_WORKFLOW.replace("  pull_request:", ""),
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release workflow without pull_request trigger should fail");

        assert!(error.to_string().contains("must run on pull_request"));
    }

    #[test]
    fn rejects_release_workflow_publish_commands() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);

        let command = "dist publish";
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            &RELEASE_PLAN_WORKFLOW.replace("dist plan", command),
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release publishing command should fail before release milestone");

        assert!(error.to_string().contains(command));
    }

    #[test]
    fn rejects_release_workflow_without_dist_plan() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            &RELEASE_PLAN_WORKFLOW.replace("dist plan", "cargo dist --help"),
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release workflow without dist plan should fail");

        assert!(error.to_string().contains("dist plan"));
    }

    #[test]
    fn rejects_release_workflow_without_dist_build() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            &RELEASE_PLAN_WORKFLOW.replace("dist build", "cargo build"),
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release workflow without dist build should fail");

        assert!(error.to_string().contains("build release artifacts"));
    }

    #[test]
    fn rejects_release_workflow_without_dist_host() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            &RELEASE_PLAN_WORKFLOW.replace("dist host", "gh release upload"),
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release workflow without dist host should fail");

        assert!(error.to_string().contains("host release artifacts"));
    }

    #[test]
    fn rejects_release_workflow_without_github_release_creation() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(temp.path(), "dist-workspace.toml", DIST_CONFIG);
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            &RELEASE_PLAN_WORKFLOW.replace("gh release create", "gh release view"),
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release workflow without GitHub Release creation should fail");

        assert!(error.to_string().contains("GitHub Release"));
    }

    #[test]
    fn rejects_release_cargo_dist_version_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                "cargo-dist-version = \"0.32.0\"",
                "cargo-dist-version = \"0.33.0\"",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release cargo-dist-version drift should fail");

        assert!(error.to_string().contains("cargo-dist-version"));
    }

    #[test]
    fn rejects_release_planning_mode_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace("pr-run-mode = \"plan\"", "pr-run-mode = \"build\""),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release planning mode drift should fail");

        assert!(
            error
                .to_string()
                .contains("release planning issue(s) found")
        );
    }

    #[test]
    fn rejects_release_allow_dirty_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                "allow-dirty = [\"ci\"]",
                "allow-dirty = [\"ci\", \"source\"]",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release allow-dirty drift should fail");

        assert!(error.to_string().contains("allow-dirty"));
    }

    #[test]
    fn rejects_release_packages_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                "packages = [\"kply-cli\"]",
                "packages = [\"kply-cli\", \"xtask\"]",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release package drift should fail");

        assert!(error.to_string().contains("kply-cli"));
    }

    #[test]
    fn rejects_release_installer_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace("installers = [\"shell\"]", "installers = [\"powershell\"]"),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release installer drift should fail");

        assert!(error.to_string().contains("shell installer"));
    }

    #[test]
    fn rejects_release_checksum_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace("checksum = \"sha256\"", "checksum = false"),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release checksum drift should fail");

        assert!(error.to_string().contains("sha256 checksums"));
    }

    #[test]
    fn rejects_release_attestation_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace("github-attestations = true", "github-attestations = false"),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release attestation drift should fail");

        assert!(error.to_string().contains("github-attestations"));
    }

    #[test]
    fn rejects_release_attestation_missing() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace("github-attestations = true\n", ""),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("missing release attestation setting should fail");

        assert!(error.to_string().contains("github-attestations"));
    }

    #[test]
    fn rejects_release_linux_x64_target_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                DIST_TARGETS_LINE,
                "targets = [\"aarch64-unknown-linux-gnu\", \"x86_64-unknown-linux-musl\", \"aarch64-unknown-linux-musl\", \"x86_64-apple-darwin\", \"aarch64-apple-darwin\"]",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release Linux x64 target drift should fail");

        assert!(error.to_string().contains("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn rejects_release_linux_arm64_target_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                DIST_TARGETS_LINE,
                "targets = [\"x86_64-unknown-linux-gnu\", \"x86_64-unknown-linux-musl\", \"aarch64-unknown-linux-musl\", \"x86_64-apple-darwin\", \"aarch64-apple-darwin\"]",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release Linux arm64 target drift should fail");

        assert!(error.to_string().contains("aarch64-unknown-linux-gnu"));
    }

    #[test]
    fn rejects_release_linux_x64_portable_target_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                DIST_TARGETS_LINE,
                "targets = [\"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-gnu\", \"aarch64-unknown-linux-musl\", \"x86_64-apple-darwin\", \"aarch64-apple-darwin\"]",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release Linux x64 portable target drift should fail");

        assert!(error.to_string().contains("x86_64-unknown-linux-musl"));
    }

    #[test]
    fn rejects_release_linux_arm64_portable_target_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                DIST_TARGETS_LINE,
                "targets = [\"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-gnu\", \"x86_64-unknown-linux-musl\", \"x86_64-apple-darwin\", \"aarch64-apple-darwin\"]",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release Linux arm64 portable target drift should fail");

        assert!(error.to_string().contains("aarch64-unknown-linux-musl"));
    }

    #[test]
    fn rejects_release_macos_x64_target_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                DIST_TARGETS_LINE,
                "targets = [\"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-gnu\", \"x86_64-unknown-linux-musl\", \"aarch64-unknown-linux-musl\", \"aarch64-apple-darwin\"]",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release macOS x64 target drift should fail");

        assert!(error.to_string().contains("x86_64-apple-darwin"));
    }

    #[test]
    fn rejects_release_macos_arm64_target_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let dist_path = write_source(
            temp.path(),
            "dist-workspace.toml",
            &DIST_CONFIG.replace(
                DIST_TARGETS_LINE,
                "targets = [\"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-gnu\", \"x86_64-unknown-linux-musl\", \"aarch64-unknown-linux-musl\", \"x86_64-apple-darwin\"]",
            ),
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/release.yml",
            RELEASE_PLAN_WORKFLOW,
        );

        let error = check_release_planning_inner(&dist_path, &workflow_path)
            .expect_err("release macOS arm64 target drift should fail");

        assert!(error.to_string().contains("aarch64-apple-darwin"));
    }

    #[test]
    fn accepts_pinned_rust_toolchain_and_matching_ci() {
        let temp = TempDir::new().expect("temp dir should be created");
        let toolchain_path = write_source(
            temp.path(),
            "rust-toolchain.toml",
            r#"
[toolchain]
channel = "1.95.0"
components = ["clippy", "rustfmt"]
"#,
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            "toolchain: 1.95.0\n",
        );

        check_toolchain_pin_inner(&toolchain_path, &workflow_path, "1.95.0")
            .expect("matching toolchain pin should pass");
    }

    #[test]
    fn rejects_unpinned_rust_toolchain_channel() {
        let temp = TempDir::new().expect("temp dir should be created");
        let toolchain_path = write_source(
            temp.path(),
            "rust-toolchain.toml",
            r#"
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
"#,
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            "toolchain: 1.95.0\n",
        );

        let error = check_toolchain_pin_inner(&toolchain_path, &workflow_path, "1.95.0")
            .expect_err("unpinned toolchain channel should fail");

        assert!(error.to_string().contains("toolchain pin issue(s) found"));
    }

    #[test]
    fn rejects_rust_toolchain_missing_required_components() {
        let temp = TempDir::new().expect("temp dir should be created");
        let toolchain_path = write_source(
            temp.path(),
            "rust-toolchain.toml",
            r#"
[toolchain]
channel = "1.95.0"
components = ["rustfmt"]
"#,
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            "toolchain: 1.95.0\n",
        );

        let error = check_toolchain_pin_inner(&toolchain_path, &workflow_path, "1.95.0")
            .expect_err("missing toolchain component should fail");

        assert!(error.to_string().contains("toolchain pin issue(s) found"));
    }

    #[test]
    fn rejects_ci_toolchain_drift() {
        let temp = TempDir::new().expect("temp dir should be created");
        let toolchain_path = write_source(
            temp.path(),
            "rust-toolchain.toml",
            r#"
[toolchain]
channel = "1.95.0"
components = ["clippy", "rustfmt"]
"#,
        );
        let workflow_path = write_nested_source(
            temp.path(),
            ".github/workflows/ci.yml",
            "toolchain: stable\n",
        );

        let error = check_toolchain_pin_inner(&toolchain_path, &workflow_path, "1.95.0")
            .expect_err("CI toolchain drift should fail");

        assert!(error.to_string().contains("toolchain pin issue(s) found"));
    }

    #[test]
    fn matches_workflow_toolchain_key_with_whitespace() {
        let workflow = "      toolchain: 1.95.0\n";

        assert!(workflow_installs_toolchain(workflow, "1.95.0"));
    }

    #[test]
    fn rejects_workflow_toolchain_mentions_outside_key() {
        let workflow = "name: toolchain: 1.95.0\n";

        assert!(!workflow_installs_toolchain(workflow, "1.95.0"));
    }

    fn test_workspace_crates() -> &'static [WorkspaceCrate] {
        &[
            WorkspaceCrate {
                name: "kply-cli",
                path: "crates/kply-cli",
            },
            WorkspaceCrate {
                name: "xtask",
                path: "crates/xtask",
            },
        ]
    }

    fn write_source(directory: &Path, filename: &str, source: &str) -> std::path::PathBuf {
        let source_path = directory.join(filename);
        fs::write(&source_path, source).expect("source fixture should be written");
        source_path
    }

    fn write_crate_manifests(root: &Path, license_line: &str) {
        for workspace_crate in test_workspace_crates() {
            write_nested_source(
                root,
                &format!("{}/Cargo.toml", workspace_crate.path),
                &format!(
                    "[package]\nname = \"{}\"\n{}\n",
                    workspace_crate.name, license_line
                ),
            );
        }
    }

    fn write_nested_source(root: &Path, path: &str, source: &str) -> std::path::PathBuf {
        let source_path = root.join(path);
        let parent = source_path
            .parent()
            .expect("nested source path should have parent");
        fs::create_dir_all(parent).expect("nested source parent should be created");
        fs::write(&source_path, source).expect("nested source fixture should be written");
        source_path
    }
}
