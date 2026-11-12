//! The module tree of one crate, read from the source files that its root reaches.

use crate::Error;
use crate::scan::{Reference, Scan};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// One source file and the paths that it names.
pub(crate) struct Source {
    pub file: PathBuf,
    pub references: Vec<Reference>,
}

/// Every source file of one crate.
pub(crate) struct Tree {
    /// Names of the modules that the crate root declares.
    pub top: BTreeSet<String>,
    pub sources: Vec<Source>,
}

impl Tree {
    /// Read the crate whose root file is `root`, following each `mod` declaration to its file.
    pub(crate) fn read(root: &Path) -> Result<Self, Error> {
        let mut tree = Self {
            top: BTreeSet::new(),
            sources: Vec::new(),
        };
        let mut pending = vec![(root.to_owned(), Vec::new(), root.with_file_name(""))];
        while let Some((file, module, children)) = pending.pop() {
            let text = match fs::read_to_string(&file) {
                Ok(text) => text,
                Err(source) => return Err(Error::Read { file, source }),
            };
            let syntax = match syn::parse_file(&text) {
                Ok(syntax) => syntax,
                Err(source) => return Err(Error::Parse { file, source }),
            };
            let scan = Scan::new(&syntax, module, children);
            if let Some(line) = scan.relocated {
                return Err(Error::PathAttribute { file, line });
            }
            for declared in scan.modules {
                if let [name] = declared.module.as_slice() {
                    tree.top.insert(name.clone());
                }
                if let Some(children) = declared.children {
                    let nested = children.join("mod.rs");
                    let file = if nested.is_file() {
                        nested
                    } else {
                        children.with_extension("rs")
                    };
                    pending.push((file, declared.module, children));
                }
            }
            tree.sources.push(Source {
                file,
                references: scan.references,
            });
        }
        Ok(tree)
    }
}
