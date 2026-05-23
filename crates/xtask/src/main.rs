//! Repository automation placeholder for Kply development tasks.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_owned());

    match command.as_str() {
        "help" => {
            println!("available tasks:");
            println!("  check-crate-inventory-docs verify docs list workspace crates");
            println!("  check-license-files verify Apache-2.0 license and notice files");
            println!("  check-module-docs  verify crate source files start with module docs");
            println!("  check-placeholder-docs verify public docs describe placeholder status");
            println!("  check-placeholders verify product crates expose placeholder markers only");
            println!("  help               print this message");
            println!("  validate           print the validation command list");
        }
        "check-crate-inventory-docs" => {
            check_crate_inventory_docs()?;
        }
        "check-license-files" => {
            check_license_files()?;
        }
        "check-module-docs" => {
            check_module_docs()?;
        }
        "check-placeholder-docs" => {
            check_placeholder_docs()?;
        }
        "check-placeholders" => {
            check_placeholders()?;
        }
        "validate" => {
            println!("cargo fmt --all -- --check");
            println!("cargo check --all-targets --all-features --locked");
            println!("cargo clippy --all-targets --all-features --locked -- -D warnings");
            println!("cargo test --all-targets --all-features --locked");
            println!("cargo xtask check-crate-inventory-docs");
            println!("cargo xtask check-license-files");
            println!("cargo xtask check-module-docs");
            println!("cargo xtask check-placeholder-docs");
            println!("cargo xtask check-placeholders");
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

fn check_crate_inventory_docs() -> Result<()> {
    let doc_paths = ["AGENTS.md", "CONTRIBUTING.md", "crates/README.md"];

    check_crate_inventory_docs_inner("Cargo.toml".as_ref(), doc_paths, workspace_crates())
}

fn check_license_files() -> Result<()> {
    check_license_files_inner(
        "LICENSE".as_ref(),
        "NOTICE".as_ref(),
        "Cargo.toml".as_ref(),
        workspace_crates(),
    )
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
            .is_some_and(|workspace| workspace);

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

fn parse_toml_file(path: &Path) -> Result<toml::Value> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading TOML file {}", path.display()))?;
    toml::from_str(&source).with_context(|| format!("parsing TOML file {}", path.display()))
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
    // Product crates are intentionally fixed while the scaffold is placeholder-only.
    // CLI, test, and xtask crates need real support code to enforce the scaffold.
    let product_crates = [
        "crates/kply-checks/src/lib.rs",
        "crates/kply-config/src/lib.rs",
        "crates/kply-core/src/lib.rs",
        "crates/kply-k8s/src/lib.rs",
        "crates/kply-routing/src/lib.rs",
    ];

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
            required_phrases: vec!["placeholders only".into(), "future Kply session".into()],
        },
        DocExpectation {
            path: "docs/architecture.md".into(),
            required_phrases: vec![
                "kply CLI placeholder".into(),
                "Real session planning and Kubernetes execution".into(),
            ],
        },
        DocExpectation {
            path: "docs/product.md".into(),
            required_phrases: vec![
                "roadmap hypothesis, not implemented behavior".into(),
                "placeholder-only".into(),
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
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::{
        DocExpectation, WorkspaceCrate, check_crate_inventory_docs_inner, check_docs_contain,
        check_license_files_inner, check_placeholder_sources, collect_workspace_members,
        contains_crate_name, has_non_placeholder_public_item, has_placeholder_marker,
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
