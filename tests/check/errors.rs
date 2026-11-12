//! Rules and sources on which the check gives no verdict.

use crate::fixture::{Fixture, RULES, manifest};
use cargo_layers::Error;

#[test]
fn a_rules_table_that_is_stale_malformed_or_absent_is_an_error() {
    let error = |rules: &str| Fixture::new(rules).check().unwrap_err();

    let stale = error("order = [\"cli\", \"gone\"]\n");
    assert!(
        matches!(&stale, Error::UnknownModule { module, .. } if module == "gone"),
        "{stale:?}"
    );
    let stale = error("restrict = [{ paths = [\"clap::Parser\"], to = [\"item\"] }]\n");
    assert!(
        matches!(&stale, Error::UnknownModule { module, .. } if module == "item"),
        "{stale:?}"
    );

    for (rules, reason) in [
        (
            "order = [\"cli\", [\"app\", \"cli\"]]\n",
            "`order` lists `cli` twice",
        ),
        (
            "order = [\"cli\"]\ndeny = [\"app\"]\n",
            "unknown field `deny`",
        ),
        (
            "order = [\"cli\", 2]\n",
            "a module name or an array of module names in `order`",
        ),
        (
            "restrict = [{ paths = [\"super::Parser\"], to = [\"cli\"] }]\n",
            "`super::Parser` in `restrict` is not a path",
        ),
        (
            "restrict = [{ paths = [\"clap::\"], to = [\"cli\"] }]\n",
            "`clap::` in `restrict` is not a path",
        ),
    ] {
        let invalid = error(rules);
        let Error::Rules { package, source } = &invalid else {
            panic!("{rules}: {invalid:?}");
        };
        assert_eq!(package, "fixture");
        assert!(source.to_string().contains(reason), "{rules}: {source}");
    }

    let unruled = Fixture::empty();
    unruled.write(
        "Cargo.toml",
        "[package]\nname = \"free\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    );
    unruled.write("src/lib.rs", "");
    let missing = unruled.check().unwrap_err();
    assert!(matches!(missing, Error::NoRules), "{missing:?}");
}

#[test]
fn sources_that_the_check_cannot_follow_are_errors() {
    let error = |file: &str, source: &str| {
        let fixture = Fixture::new(RULES);
        fixture.write(file, source);
        fixture.check().unwrap_err()
    };

    let invalid = error("src/model/item.rs", "struct Item;\nfn broken( {}\n");
    assert!(
        matches!(&invalid, Error::Parse { file, .. } if file.ends_with("src/model/item.rs")),
        "{invalid:?}"
    );
    assert!(invalid.to_string().ends_with("item.rs:2"), "{invalid}");

    let missing = error("src/model.rs", "mod item;\nmod lost;\n");
    assert!(
        matches!(&missing, Error::Read { file, .. } if file.ends_with("src/model/lost.rs")),
        "{missing:?}"
    );

    let relocated = error(
        "src/model.rs",
        "mod item;\n#[path = \"elsewhere.rs\"]\nmod moved;\n",
    );
    assert!(
        matches!(&relocated, Error::PathAttribute { file, line: 2 } if file.ends_with("src/model.rs")),
        "{relocated:?}"
    );

    let fixture = Fixture::new(RULES);
    fixture.write(
        "Cargo.toml",
        &manifest("fixture", RULES).replace("2024", "2015"),
    );
    let edition = fixture.check().unwrap_err();
    assert!(matches!(edition, Error::Edition { .. }), "{edition:?}");
}
