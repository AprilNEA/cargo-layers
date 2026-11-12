//! The rules of one package's `[package.metadata.layers]` table.

use crate::scan::{Reference, Site};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// The rules of one package. A layer is a module that the crate root declares.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rules {
    /// Each layer and the position of its `order` entry. Position 0 is the top.
    #[serde(default, deserialize_with = "positions")]
    order: BTreeMap<String, usize>,
    #[serde(default)]
    restrict: Vec<Restriction>,
}

/// One entry of `order`: a layer, or layers that do not reference each other.
#[derive(Deserialize)]
#[serde(
    untagged,
    expecting = "a module name or an array of module names in `order`"
)]
enum Entry {
    Layer(String),
    Peers(Vec<String>),
}

fn positions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, usize>, D::Error> {
    let mut order = BTreeMap::new();
    for (position, entry) in Vec::<Entry>::deserialize(deserializer)?
        .into_iter()
        .enumerate()
    {
        let layers = match entry {
            Entry::Layer(layer) => vec![layer],
            Entry::Peers(layers) => layers,
        };
        for layer in layers {
            if order.contains_key(&layer) {
                return Err(D::Error::custom(format!("`order` lists `{layer}` twice")));
            }
            order.insert(layer, position);
        }
    }
    Ok(order)
}

/// Paths that only the modules in `to` may name.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Restriction {
    paths: Vec<Restricted>,
    to: Vec<String>,
}

/// The segments of a restricted path, which starts with a crate name or `crate`.
#[derive(Deserialize)]
#[serde(try_from = "String")]
struct Restricted(Vec<String>);

impl TryFrom<String> for Restricted {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let segments = text.split("::").map(str::to_owned).collect::<Vec<_>>();
        let valid = segments.iter().enumerate().all(|(index, segment)| {
            syn::parse_str::<syn::Ident>(segment).is_ok() || (index == 0 && segment == "crate")
        });
        if !valid {
            return Err(format!(
                "`{text}` in `restrict` is not a path that starts with a crate name or `crate`"
            ));
        }
        Ok(Self(segments))
    }
}

impl Rules {
    /// The modules that the rules name.
    pub(crate) fn modules(&self) -> impl Iterator<Item = &str> {
        let restricted = self.restrict.iter().flat_map(|restriction| &restriction.to);
        self.order.keys().chain(restricted).map(String::as_str)
    }

    /// The rules that `reference` breaks, one message for each.
    pub(crate) fn breaches(&self, reference: &Reference) -> Vec<String> {
        let path = reference.absolute();
        let restricted = self
            .restrict
            .iter()
            .filter_map(|restriction| restriction.breach(reference, &path));
        self.layering(reference, &path)
            .into_iter()
            .chain(restricted)
            .collect()
    }

    fn layering(&self, reference: &Reference, path: &[String]) -> Option<String> {
        let from = reference.module.first()?;
        let position = self.order.get(from)?;
        let [root, inside @ ..] = path else {
            return None;
        };
        if root != "crate" {
            return None;
        }
        let written = reference.written();
        match inside.first() {
            Some(to) if to != from => {
                let side = match self.order.get(to)?.cmp(position) {
                    Ordering::Less => "above",
                    Ordering::Equal => "beside",
                    Ordering::Greater => return None,
                };
                Some(format!(
                    "`{from}` references `{to}`, a layer {side} it: `{written}`"
                ))
            }
            // The crate root exposes every layer by its short name.
            None if reference.site != Site::Path
                && self
                    .order
                    .iter()
                    .any(|(layer, other)| layer != from && other <= position) =>
            {
                Some(format!(
                    "`{from}` imports the crate root, which holds layers that `{from}` must not reference: `{written}`"
                ))
            }
            _ => None,
        }
    }
}

impl Restriction {
    fn breach(&self, reference: &Reference, path: &[String]) -> Option<String> {
        let from = reference.module.first();
        if from.is_some_and(|from| self.to.contains(from)) {
            return None;
        }
        let written = reference.written();
        let only = match self.to.as_slice() {
            [] => "no module".to_owned(),
            modules => modules
                .iter()
                .map(|module| format!("`{module}`"))
                .collect::<Vec<_>>()
                .join(", "),
        };
        self.paths
            .iter()
            .find_map(|Restricted(restricted)| match reference.site {
                Site::Import | Site::Path if path.starts_with(restricted) => {
                    Some(format!("`{written}` is restricted to {only}"))
                }
                Site::Glob if path.starts_with(restricted) || restricted.starts_with(path) => {
                    Some(format!(
                        "`{written}` brings `{}` into scope, which is restricted to {only}",
                        restricted.join("::")
                    ))
                }
                _ => None,
            })
    }
}
