//! Which packages and targets the check reads.

use crate::fixture::{Fixture, ROOT, RULES, manifest};
use std::fs;
use std::path::Path;

#[test]
fn a_package_without_a_library_is_checked_through_its_only_binary() {
    let fixture = Fixture::new(RULES);
    fs::remove_file(fixture.root().join("src/lib.rs")).unwrap();
    fixture.write("src/main.rs", &format!("{ROOT}fn main() {{}}\n"));
    fixture.write("src/model/item.rs", "use crate::cli::Flag;\n");

    let violations = fixture.check().unwrap();

    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].file, Path::new("src/model/item.rs"));
}

#[test]
fn only_workspace_packages_with_rules_are_checked_and_files_are_relative_to_the_workspace() {
    let workspace = Fixture::empty();
    workspace.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"free\", \"ruled\"]\nresolver = \"3\"\n",
    );
    workspace.write(
        "free/Cargo.toml",
        "[package]\nname = \"free\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    );
    workspace.write("free/src/lib.rs", "mod cli;\nmod model;\n");
    workspace.write("free/src/cli.rs", "");
    workspace.write("free/src/model.rs", "use crate::cli::Flag;\n");
    workspace.write(
        "ruled/Cargo.toml",
        &manifest("ruled", "order = [\"cli\", \"model\"]\n"),
    );
    workspace.write("ruled/src/lib.rs", "mod cli;\nmod model;\n");
    workspace.write("ruled/src/cli.rs", "");
    workspace.write("ruled/src/model.rs", "\nuse crate::cli::Flag;\n");

    let violations = workspace.check().unwrap();

    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].file, Path::new("ruled/src/model.rs"));
    assert_eq!(violations[0].line, 2);
}
