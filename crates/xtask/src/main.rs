//! Repository automation placeholder for Kply development tasks.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_owned());

    match command.as_str() {
        "help" => {
            println!("available tasks:");
            println!("  check-module-docs  verify crate source files start with module docs");
            println!("  check-placeholder-docs verify public docs describe placeholder status");
            println!("  check-placeholders verify product crates expose placeholder markers only");
            println!("  help               print this message");
            println!("  validate           print the validation command list");
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
        let source = std::fs::read_to_string(&doc.path)?;

        for phrase in doc.required_phrases {
            if !source.contains(&phrase) {
                missing_phrases.push((doc.path.clone(), phrase));
            }
        }
    }

    if !missing_phrases.is_empty() {
        for (path, phrase) in &missing_phrases {
            eprintln!(
                "placeholder documentation phrase missing in {}: {phrase}",
                path.display()
            );
        }
        bail!(
            "{} placeholder documentation phrase(s) missing",
            missing_phrases.len()
        );
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
        DocExpectation, check_docs_contain, check_placeholder_sources,
        has_non_placeholder_public_item, has_placeholder_marker,
    };

    const PLACEHOLDER_SOURCE: &str = "\
//! Core domain placeholders for future Kply session primitives.

/// Placeholder marker for the future core session model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorePlaceholder;
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

    fn write_source(directory: &Path, filename: &str, source: &str) -> std::path::PathBuf {
        let source_path = directory.join(filename);
        fs::write(&source_path, source).expect("source fixture should be written");
        source_path
    }
}
