//! Output and exit codes of the `cargo layers` command.

use crate::fixture::{Fixture, RULES};
use std::process::Command;

#[test]
fn the_command_reports_violations_on_stderr_and_separates_them_from_errors_by_exit_code() {
    let run = |fixture: &Fixture, arguments: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-layers"))
            .current_dir(fixture.root())
            .arg("layers")
            .args(arguments)
            .output()
            .unwrap();
        (
            output.status.code(),
            String::from_utf8(output.stderr).unwrap(),
        )
    };

    let fixture = Fixture::new(RULES);
    assert_eq!(run(&fixture, &[]), (Some(0), String::new()));

    fixture.write("src/model/item.rs", "use crate::cli::Flag;\n");
    let (code, stderr) = run(&fixture, &["--manifest-path", "Cargo.toml"]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(
        stderr.contains("item.rs:1: `model` references `cli`, a layer above it"),
        "{stderr}"
    );
    assert!(
        stderr.ends_with("error: 1 module layer violation(s)\n"),
        "{stderr}"
    );

    let (code, stderr) = run(&Fixture::new("order = [\"gone\"]\n"), &[]);
    assert_eq!(code, Some(2), "{stderr}");
    assert!(stderr.contains("names `gone`"), "{stderr}");
}
