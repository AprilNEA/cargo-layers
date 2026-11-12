# cargo-layers

`cargo layers` checks which modules of a crate may reference each other. A package states its rules in `Cargo.toml`. The check parses the package's source files and reports the file and line of each reference that breaks a rule.

## Usage

Install the command, then run it in a package or a workspace:

```sh
cargo install --locked cargo-layers
cargo layers [--manifest-path <PATH>]
```

A task runner can depend on the crate and call the same check as a library:

```sh
cargo add cargo-layers
```

```rust,no_run
fn main() -> Result<(), cargo_layers::Error> {
    for violation in cargo_layers::check(None)? {
        eprintln!("{violation}");
    }
    Ok(())
}
```

## Rules

```toml
[package.metadata.layers]
order = ["cli", "app", ["slack", "output"], "model"]
restrict = [{ paths = ["clap::Parser"], to = ["cli"] }]
```

`order` lists layers from top to bottom. A layer is a module that the crate root declares, together with every module inside it. A layer may reference only the layers in later entries. An array entry holds layers that must not reference each other. A module that `order` does not list is unconstrained, and every layer may reference it.

`restrict` lists paths that only the modules in `to` may name. A path starts with a crate name or `crate` and covers every item inside it: `clap::Parser` covers `clap::Parser::parse`, and `tokio` covers the whole crate. A glob import that brings a restricted path into scope, such as `use clap::*`, breaks the rule too.

With the rules above, a reference from `model` to `cli` prints:

```text
src/model/files.rs:127: `model` references `cli`, a layer above it: `crate::cli::Cli`
error: 1 module layer violation(s)
```

| Exit code | Meaning |
| --- | --- |
| 0 | Every rule holds. |
| 1 | A reference breaks a rule. |
| 2 | The check cannot give a verdict. |

The check gives no verdict when no workspace package has a rules table, when a rules table is malformed or names a module that the crate root does not declare, or when a source file is missing or does not parse. A renamed layer therefore cannot silently drop its rules.

## What the check reads

The check covers every workspace package that has a `[package.metadata.layers]` table. In each package it reads the library target, or else the only binary target, and follows each `mod` declaration to its file. It reads every module regardless of `cfg` attributes, so `cfg(test)` code is checked.

A reference is a path in one of these places:

- a `use` declaration, including grouped, renamed, and glob imports
- a type, a trait bound, or an expression
- the arguments of a macro call or an attribute, including a path in a string such as `#[serde(with = "crate::app::format")]`

The check resolves `crate`, `self`, and `super` to a path from the crate root. A layer that imports the crate root, such as `use crate::*`, breaks `order` when the root holds a layer that the importing layer must not reference.

## Limits

The check compares paths as the source writes them. It does not resolve names, so it misses a reference that reaches a module under another name:

- a renamed parent: after `use clap as c;` or `extern crate clap as c;`, `c::Parser` is not matched
- a re-export: when a module outside `order` re-exports an item of a layer, a reference to the re-export is not matched
- a path that exists only after macro expansion, such as code that a derive macro generates

Rules name top-level modules only, so they cannot constrain a nested module on its own. The check reads one target of each package, so it does not read integration tests, examples, benchmarks, or a binary target beside a library. A module with a `#[path]` attribute and a package on the 2015 edition are errors, not silent gaps.

## License

Licensed under either the MIT License (`LICENSE-MIT`) or the Apache License, Version 2.0 (`LICENSE-APACHE`), at your option.
