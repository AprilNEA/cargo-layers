#![doc = include_str!("../README.md")]

mod rules;
mod scan;
mod tree;

use cargo_metadata::{Edition, MetadataCommand, Package, TargetKind};
use rules::Rules;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::{fmt, io};
use tree::Tree;

/// Key of the rules table under `[package.metadata]`.
const TABLE: &str = "layers";

/// A reference that breaks a rule.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    /// Source file of the reference, relative to the workspace root.
    pub file: PathBuf,
    /// One-based line of the reference.
    pub line: usize,
    /// The rule that the reference breaks.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            file,
            line,
            message,
        } = self;
        write!(formatter, "{}:{line}: {message}", file.display())
    }
}

/// Why a check did not run to the end.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// `cargo metadata` failed.
    #[error("Read Cargo metadata")]
    Metadata(#[from] cargo_metadata::Error),
    /// No workspace package declares rules, so a check would verify nothing.
    #[error("No workspace package has a `[package.metadata.{TABLE}]` table")]
    NoRules,
    /// A rules table has an unknown key, a wrong shape, a repeated layer, or an invalid path.
    #[error("Read `[package.metadata.{TABLE}]` of package `{package}`")]
    Rules {
        /// Package that owns the table.
        package: String,
        /// What the table gets wrong.
        #[source]
        source: serde_json::Error,
    },
    /// A rule names a module that the crate root does not declare, so the rule constrains nothing.
    #[error(
        "`[package.metadata.{TABLE}]` of package `{package}` names `{module}`, which the crate root does not declare"
    )]
    UnknownModule {
        /// Package that owns the table.
        package: String,
        /// Name that matches no top-level module.
        module: String,
    },
    /// The package has no library target and does not have exactly one binary target.
    #[error("Package `{package}` has neither a library target nor exactly one binary target")]
    NoRoot {
        /// Package without a crate root to check.
        package: String,
    },
    /// The package uses the 2015 edition, where a path into the crate does not start with `crate`.
    #[error("Package `{package}` uses the 2015 edition, which the check does not support")]
    Edition {
        /// Package on the 2015 edition.
        package: String,
    },
    /// A source file is missing or unreadable.
    #[error("Read {}", file.display())]
    Read {
        /// File that a `mod` declaration names.
        file: PathBuf,
        /// Why the read failed.
        #[source]
        source: io::Error,
    },
    /// A source file is not valid Rust.
    #[error("Parse {}:{}", file.display(), source.span().start().line)]
    Parse {
        /// File that failed to parse.
        file: PathBuf,
        /// What the parser rejected.
        #[source]
        source: syn::Error,
    },
    /// A module has a `#[path]` attribute, so its file is not where the check looks for it.
    #[error("{}:{line}: a module with a `#[path]` attribute is not supported", file.display())]
    PathAttribute {
        /// File that declares the module.
        file: PathBuf,
        /// One-based line of the attribute.
        line: usize,
    },
}

/// Check every workspace package that has a `[package.metadata.layers]` table.
///
/// `manifest` is the `Cargo.toml` of the workspace or of one of its packages. Without one, Cargo
/// finds the manifest from the current directory.
///
/// The violations are sorted by file and line. An empty list means that every rule holds.
///
/// # Errors
///
/// Returns an [`Error`] when the check cannot give a verdict: Cargo metadata, a rules table, or a
/// source file is missing or invalid, or the package has a shape that the check does not support.
pub fn check(manifest: Option<&Path>) -> Result<Vec<Violation>, Error> {
    let mut command = MetadataCommand::new();
    command.no_deps();
    if let Some(manifest) = manifest {
        command.manifest_path(manifest);
    }
    let metadata = command.exec()?;
    let ruled = metadata
        .workspace_packages()
        .into_iter()
        .filter_map(|package| Some((package, package.metadata.get(TABLE)?)))
        .collect::<Vec<_>>();
    if ruled.is_empty() {
        return Err(Error::NoRules);
    }
    let mut violations = Vec::new();
    for (package, table) in ruled {
        let name = || package.name.to_string();
        let rules = Rules::deserialize(table).map_err(|source| Error::Rules {
            package: name(),
            source,
        })?;
        let tree = Tree::read(root(package)?)?;
        if let Some(module) = rules.modules().find(|module| !tree.top.contains(*module)) {
            return Err(Error::UnknownModule {
                package: name(),
                module: module.to_owned(),
            });
        }
        for source in &tree.sources {
            let file = source
                .file
                .strip_prefix(&metadata.workspace_root)
                .unwrap_or(&source.file);
            for reference in &source.references {
                for message in rules.breaches(reference) {
                    violations.push(Violation {
                        file: file.to_owned(),
                        line: reference.line,
                        message,
                    });
                }
            }
        }
    }
    violations.sort();
    Ok(violations)
}

/// The root file of the crate to check: the library target, or else the only binary target.
fn root(package: &Package) -> Result<&Path, Error> {
    let name = || package.name.to_string();
    if package.edition == Edition::E2015 {
        return Err(Error::Edition { package: name() });
    }
    let kind = |kinds: &[TargetKind]| {
        let mut targets = package
            .targets
            .iter()
            .filter(move |target| target.kind.iter().any(|kind| kinds.contains(kind)));
        let first = targets.next();
        first.filter(|_| targets.next().is_none())
    };
    let library = [
        TargetKind::Lib,
        TargetKind::RLib,
        TargetKind::DyLib,
        TargetKind::CDyLib,
        TargetKind::StaticLib,
        TargetKind::ProcMacro,
    ];
    kind(&library)
        .or_else(|| kind(&[TargetKind::Bin]))
        .map(|target| target.src_path.as_std_path())
        .ok_or_else(|| Error::NoRoot { package: name() })
}
