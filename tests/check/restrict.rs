//! Paths that `restrict` confines to some modules.

use crate::fixture::{Case, assert_cases};

#[test]
fn a_restricted_path_is_reported_outside_the_modules_it_is_restricted_to() {
    assert_cases(&[
        Case {
            name: "qualified derive",
            file: "src/app/job.rs",
            source: "#[derive(Debug, clap::Parser)]\nstruct Job;\n",
            expected: &[(1, "`clap::Parser` is restricted to `cli`")],
        },
        Case {
            name: "renamed member of a grouped import",
            file: "src/model/item.rs",
            source: "use clap::{Parser as Derive, ValueEnum};\n",
            expected: &[(1, "`clap::Parser` is restricted to `cli`")],
        },
        Case {
            name: "glob import of the parent path",
            file: "src/slack.rs",
            source: "mod http;\nuse clap::*;\n",
            expected: &[(
                2,
                "`clap::*` brings `clap::Parser` into scope, which is restricted to `cli`",
            )],
        },
        Case {
            name: "bound and associated item, in a module outside `order`",
            file: "src/auth.rs",
            source: "fn parse<T: clap::Parser>() -> T {\n    <T as clap::Parser>::parse()\n}\n",
            expected: &[
                (1, "`clap::Parser` is restricted to `cli`"),
                (2, "`clap::Parser::parse` is restricted to `cli`"),
            ],
        },
        Case {
            name: "crate root",
            file: "src/lib.rs",
            source: "mod app;\nmod auth;\nmod cli;\nmod model;\nmod output;\nmod slack;\nuse clap::Parser;\n",
            expected: &[(7, "`clap::Parser` is restricted to `cli`")],
        },
        Case {
            name: "whole crate",
            file: "src/model/item.rs",
            source: "use tokio;\nfn spawn() {\n    tokio::spawn(async {});\n}\n",
            expected: &[
                (1, "`tokio` is restricted to `cli`"),
                (3, "`tokio::spawn` is restricted to `cli`"),
            ],
        },
    ]);
}

#[test]
fn a_path_that_only_resembles_a_restricted_path_is_not_reported() {
    assert_cases(&[
        Case {
            name: "module that the path is restricted to",
            file: "src/cli.rs",
            source: "use clap::Parser;\n#[derive(Parser)]\nstruct Options;\nfn spawn() {\n    tokio::spawn(async {});\n}\n",
            expected: &[],
        },
        Case {
            name: "other items of the crate and the same name in another crate",
            file: "src/output.rs",
            source: "use clap::ValueEnum;\nuse clap::builder::*;\nfn strip(_: &mut vte::Parser) {}\n",
            expected: &[],
        },
        Case {
            name: "one-segment names outside an import",
            file: "src/model/item.rs",
            source: "#[cfg(feature = \"tokio\")]\nfn local(tokio: u8) -> u8 {\n    tokio + 1\n}\n",
            expected: &[],
        },
    ]);
}
