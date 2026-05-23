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
            println!("  check-placeholders verify product crates expose placeholder markers only");
            println!("  help               print this message");
            println!("  validate           print the validation command list");
        }
        "check-module-docs" => {
            check_module_docs()?;
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
    let product_crates = [
        "crates/kply-checks/src/lib.rs",
        "crates/kply-config/src/lib.rs",
        "crates/kply-core/src/lib.rs",
        "crates/kply-k8s/src/lib.rs",
        "crates/kply-routing/src/lib.rs",
    ];
    let mut invalid_sources = Vec::new();

    for source_path in product_crates {
        let source = std::fs::read_to_string(source_path)?;

        if !has_placeholder_marker(&source) || has_non_placeholder_public_item(&source) {
            invalid_sources.push(source_path);
        }
    }

    if !invalid_sources.is_empty() {
        for source_path in &invalid_sources {
            eprintln!("product crate is not placeholder-only: {source_path}");
        }
        bail!(
            "{} product crate source file(s) are not placeholder-only",
            invalid_sources.len()
        );
    }

    Ok(())
}

fn has_placeholder_marker(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.trim_start().starts_with("pub struct ") && line.contains("Placeholder"))
}

fn has_non_placeholder_public_item(source: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        (line.starts_with("pub enum ")
            || line.starts_with("pub fn ")
            || line.starts_with("pub trait ")
            || line.starts_with("pub type ")
            || line.starts_with("pub const ")
            || line.starts_with("pub static "))
            || (line.starts_with("pub struct ") && !line.contains("Placeholder"))
    })
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
