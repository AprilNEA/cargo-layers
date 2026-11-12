//! Packages in temporary directories and a table form for the violations that the check reports.

use cargo_layers::{Error, Violation, check};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub(crate) const RULES: &str = r#"order = ["cli", "app", ["slack", "output"], "model"]
restrict = [{ paths = ["clap::Parser", "tokio"], to = ["cli"] }]
"#;

/// The crate root of `SOURCES`.
pub(crate) const ROOT: &str =
    "mod app;\nmod auth;\nmod cli;\nmod model;\nmod output;\nmod slack;\n";

/// The layers of `RULES` and `auth`, a module outside `order`.
const SOURCES: [(&str, &str); 11] = [
    ("src/lib.rs", ROOT),
    ("src/app/mod.rs", "mod job;\n"),
    ("src/app/job.rs", ""),
    ("src/auth.rs", ""),
    ("src/cli.rs", ""),
    ("src/model.rs", "mod item;\n"),
    ("src/model/item.rs", ""),
    ("src/output.rs", ""),
    ("src/slack.rs", "mod http;\n"),
    ("src/slack/http.rs", "mod retry;\n"),
    ("src/slack/http/retry.rs", ""),
];

/// A package manifest with the rules table `rules`.
pub(crate) fn manifest(name: &str, rules: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[package.metadata.layers]\n{rules}"
    )
}

/// A temporary directory that holds a package or a workspace.
pub(crate) struct Fixture(TempDir);

impl Fixture {
    pub(crate) fn empty() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    /// The package `fixture` with `SOURCES` and the rules table `rules`.
    pub(crate) fn new(rules: &str) -> Self {
        let fixture = Self::empty();
        fixture.write("Cargo.toml", &manifest("fixture", rules));
        for (file, source) in SOURCES {
            fixture.write(file, source);
        }
        fixture
    }

    pub(crate) fn root(&self) -> &Path {
        self.0.path()
    }

    pub(crate) fn write(&self, file: &str, text: &str) {
        let path = self.root().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    pub(crate) fn check(&self) -> Result<Vec<Violation>, Error> {
        check(Some(&self.root().join("Cargo.toml")))
    }
}

/// One source file to replace in the fixture, and the line and message of each violation in it.
pub(crate) struct Case {
    pub name: &'static str,
    pub file: &'static str,
    pub source: &'static str,
    pub expected: &'static [(usize, &'static str)],
}

pub(crate) fn assert_cases(cases: &[Case]) {
    for case in cases {
        let fixture = Fixture::new(RULES);
        fixture.write(case.file, case.source);
        let expected = case
            .expected
            .iter()
            .map(|(line, message)| Violation {
                file: case.file.into(),
                line: *line,
                message: (*message).to_owned(),
            })
            .collect::<Vec<_>>();
        assert_eq!(fixture.check().unwrap(), expected, "{}", case.name);
    }
}
