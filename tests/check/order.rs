//! References between layers that `order` forbids and allows.

use crate::fixture::{Case, assert_cases};

#[test]
fn a_layer_that_names_a_layer_above_or_beside_it_is_reported_at_the_reference() {
    assert_cases(&[
        Case {
            name: "import",
            file: "src/model/item.rs",
            source: "use crate::cli::Flag;\n",
            expected: &[(
                1,
                "`model` references `cli`, a layer above it: `crate::cli::Flag`",
            )],
        },
        Case {
            name: "renamed member of a grouped import, in a file two modules deep",
            file: "src/slack/http/retry.rs",
            source: "use crate::{\n    auth::Token,\n    cli::Flag as Switch,\n    model::Record,\n};\n",
            expected: &[(
                3,
                "`slack` references `cli`, a layer above it: `crate::cli::Flag`",
            )],
        },
        Case {
            name: "`self` in a grouped import",
            file: "src/model/item.rs",
            source: "use crate::app::{\n    self,\n    job::Job,\n};\n",
            expected: &[
                (
                    2,
                    "`model` references `app`, a layer above it: `crate::app`",
                ),
                (
                    3,
                    "`model` references `app`, a layer above it: `crate::app::job::Job`",
                ),
            ],
        },
        Case {
            name: "glob import",
            file: "src/model/item.rs",
            source: "use crate::slack::*;\n",
            expected: &[(
                1,
                "`model` references `slack`, a layer above it: `crate::slack::*`",
            )],
        },
        Case {
            name: "raw identifier",
            file: "src/model/item.rs",
            source: "use crate::r#cli::Flag;\n",
            expected: &[(
                1,
                "`model` references `cli`, a layer above it: `crate::cli::Flag`",
            )],
        },
        Case {
            name: "layer in the same `order` entry",
            file: "src/slack/http.rs",
            source: "mod retry;\nuse crate::output::Table;\n",
            expected: &[(
                2,
                "`slack` references `output`, a layer beside it: `crate::output::Table`",
            )],
        },
        Case {
            name: "the other direction of the same `order` entry",
            file: "src/output.rs",
            source: "fn render(_: &crate::slack::http::Response) {}\n",
            expected: &[(
                1,
                "`output` references `slack`, a layer beside it: `crate::slack::http::Response`",
            )],
        },
        Case {
            name: "`super` chain that leaves the layer",
            file: "src/model/item.rs",
            source: "fn read(_: &super::super::cli::Flag) {}\n",
            expected: &[(
                1,
                "`model` references `cli`, a layer above it: `super::super::cli::Flag`",
            )],
        },
        Case {
            name: "`super` chain from an inline module",
            file: "src/model.rs",
            source: "mod item;\nmod inner {\n    use super::super::cli::Flag;\n}\n",
            expected: &[(
                3,
                "`model` references `cli`, a layer above it: `super::super::cli::Flag`",
            )],
        },
        Case {
            name: "`cfg(test)` module",
            file: "src/model/item.rs",
            source: "#[cfg(test)]\nmod tests {\n    use crate::cli::Flag;\n}\n",
            expected: &[(
                3,
                "`model` references `cli`, a layer above it: `crate::cli::Flag`",
            )],
        },
        Case {
            name: "module in a `mod.rs` file",
            file: "src/app/mod.rs",
            source: "mod job;\nuse crate::cli::Flag;\n",
            expected: &[(
                2,
                "`app` references `cli`, a layer above it: `crate::cli::Flag`",
            )],
        },
        Case {
            name: "trait path and expression in a function body",
            file: "src/app/job.rs",
            source: "fn run() {\n    struct Local;\n    impl crate::cli::Run for Local {}\n    let _ = crate::cli::run;\n}\n",
            expected: &[
                (
                    3,
                    "`app` references `cli`, a layer above it: `crate::cli::Run`",
                ),
                (
                    4,
                    "`app` references `cli`, a layer above it: `crate::cli::run`",
                ),
            ],
        },
        Case {
            name: "macro argument",
            file: "src/model/item.rs",
            source: "fn name() -> String {\n    format!(\n        \"{:?}\",\n        crate::app::NAME\n    )\n}\n",
            expected: &[(
                4,
                "`model` references `app`, a layer above it: `crate::app::NAME`",
            )],
        },
        Case {
            name: "attribute argument",
            file: "src/model/item.rs",
            source: "#[arg(value_parser = crate::cli::parse)]\nstruct Item;\n",
            expected: &[(
                1,
                "`model` references `cli`, a layer above it: `crate::cli::parse`",
            )],
        },
        Case {
            name: "path in an attribute string",
            file: "src/output.rs",
            source: "struct Row {\n    #[serde(with = \"crate::app::format\")]\n    cell: u8,\n}\n",
            expected: &[(
                2,
                "`output` references `app`, a layer above it: `crate::app::format`",
            )],
        },
        Case {
            name: "glob import of the crate root",
            file: "src/model/item.rs",
            source: "use crate::*;\nfn read(_: &cli::Flag) {}\n",
            expected: &[(
                1,
                "`model` imports the crate root, which holds layers that `model` must not reference: `crate::*`",
            )],
        },
        Case {
            name: "glob import of the crate root through `super`",
            file: "src/model.rs",
            source: "mod item;\nuse super::*;\n",
            expected: &[(
                2,
                "`model` imports the crate root, which holds layers that `model` must not reference: `super::*`",
            )],
        },
        Case {
            name: "renamed import of the crate root",
            file: "src/output.rs",
            source: "use crate as root;\n",
            expected: &[(
                1,
                "`output` imports the crate root, which holds layers that `output` must not reference: `crate`",
            )],
        },
    ]);
}

#[test]
fn references_that_follow_the_order_are_not_reported() {
    assert_cases(&[
        Case {
            name: "layers below",
            file: "src/cli.rs",
            source: "use crate::{app::job::Job, model::item::Item, output::Table, slack::http::Client};\n",
            expected: &[],
        },
        Case {
            name: "layers below a middle layer",
            file: "src/app/job.rs",
            source: "fn run(_: crate::slack::http::Client, _: crate::output::Table, _: crate::model::Record) {}\n",
            expected: &[],
        },
        Case {
            name: "the top layer imports the crate root",
            file: "src/cli.rs",
            source: "use crate::*;\n",
            expected: &[],
        },
        Case {
            name: "a module outside `order` references the top layer",
            file: "src/auth.rs",
            source: "use crate::cli::Flag;\n",
            expected: &[],
        },
        Case {
            name: "the bottom layer references a module outside `order`",
            file: "src/model/item.rs",
            source: "use crate::auth::Token;\npub(crate) struct Item;\n",
            expected: &[],
        },
        Case {
            name: "the same layer through `crate`, `self`, and `super`",
            file: "src/slack/http/retry.rs",
            source: "use crate::slack::http::Client;\nuse self::policy::Policy;\nuse super::Response;\nuse super::super::Session;\nuse super::*;\n",
            expected: &[],
        },
        Case {
            name: "`super` from an inline module names its enclosing module",
            file: "src/model.rs",
            source: "mod item;\nmod inner {\n    use super::cli::Flag;\n}\n",
            expected: &[],
        },
        Case {
            name: "local names that match a layer",
            file: "src/model/item.rs",
            source: "mod cli {\n    pub struct Flag;\n}\nfn read(app: u8) -> u8 {\n    let _ = cli::Flag;\n    app\n}\n",
            expected: &[],
        },
        Case {
            name: "text that is not code",
            file: "src/model/item.rs",
            source: "/// See `crate::cli::Flag`.\n#[doc = \"crate::cli::Flag\"]\nfn text() -> &'static str {\n    println!(\"crate::cli::Flag\");\n    \"crate::cli::Flag\"\n}\n",
            expected: &[],
        },
    ]);
}
