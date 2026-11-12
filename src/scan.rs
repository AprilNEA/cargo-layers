//! The modules that one source file declares and the paths that it names.

use proc_macro2::{Ident, TokenStream, TokenTree};
use std::path::PathBuf;
use std::{iter, mem};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// How a source file names a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Site {
    /// A `use` declaration that imports the path.
    Import,
    /// A `use` declaration that imports every name inside the path.
    Glob,
    /// A type, an expression, a macro argument, or an attribute argument.
    Path,
}

/// One path that a source file names.
#[derive(Debug)]
pub(crate) struct Reference {
    /// Module that contains the reference, from the crate root.
    pub module: Vec<String>,
    /// Path segments as written.
    pub segments: Vec<String>,
    pub site: Site,
    /// One-based line of the reference.
    pub line: usize,
}

impl Reference {
    /// The path from `crate` when the written path starts with `crate`, `self`, or `super`;
    /// otherwise the written path, which names an external crate or a local item.
    pub(crate) fn absolute(&self) -> Vec<String> {
        let supers = self
            .segments
            .iter()
            .take_while(|segment| *segment == "super")
            .count();
        let base = match self.segments.first().map(String::as_str) {
            Some("crate") => &[],
            Some("self" | "super") => &self.module[..self.module.len().saturating_sub(supers)],
            _ => return self.segments.clone(),
        };
        iter::once("crate".to_owned())
            .chain(base.iter().cloned())
            .chain(self.segments[supers.max(1)..].iter().cloned())
            .collect()
    }

    /// The path as the source file writes it.
    pub(crate) fn written(&self) -> String {
        let path = self.segments.join("::");
        match self.site {
            Site::Glob => format!("{path}::*"),
            Site::Import | Site::Path => path,
        }
    }
}

/// A module that a source file declares.
pub(crate) struct Declared {
    /// Module path from the crate root.
    pub module: Vec<String>,
    /// Directory of the module's child module files, when the module body is in another file.
    pub children: Option<PathBuf>,
}

/// What one source file declares and names.
pub(crate) struct Scan {
    /// Module that the visitor is inside.
    module: Vec<String>,
    /// Directory of that module's child module files.
    children: PathBuf,
    pub references: Vec<Reference>,
    pub modules: Vec<Declared>,
    /// Line of the first `#[path]` attribute on a module.
    pub relocated: Option<usize>,
}

impl Scan {
    /// Scan the file of `module`, whose child module files are in `children`.
    pub(crate) fn new(file: &syn::File, module: Vec<String>, children: PathBuf) -> Self {
        let mut scan = Self {
            module,
            children,
            references: Vec::new(),
            modules: Vec::new(),
            relocated: None,
        };
        scan.visit_file(file);
        scan
    }

    fn reference(&mut self, segments: Vec<String>, site: Site, line: usize) {
        // One segment outside a `use` declaration names a local item, never a crate or a layer.
        if site != Site::Path || segments.len() > 1 {
            self.references.push(Reference {
                module: self.module.clone(),
                segments,
                site,
                line,
            });
        }
    }

    fn imports(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        let leaf = |prefix: &[String], ident: &Ident| {
            let mut path = prefix.to_vec();
            if ident != "self" {
                path.push(name(ident));
            }
            path
        };
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(name(&path.ident));
                self.imports(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(import) => {
                let path = leaf(prefix, &import.ident);
                self.reference(path, Site::Import, line(&import.ident));
            }
            syn::UseTree::Rename(import) => {
                let path = leaf(prefix, &import.ident);
                self.reference(path, Site::Import, line(&import.ident));
            }
            syn::UseTree::Glob(glob) => {
                self.reference(prefix.clone(), Site::Glob, glob.span().start().line);
            }
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    self.imports(tree, prefix);
                }
            }
        }
    }

    /// Macro and attribute arguments are unparsed tokens: read each `a::b` run as a path.
    fn tokens(&mut self, stream: TokenStream, literals: bool) {
        let trees = stream.into_iter().collect::<Vec<_>>();
        let mut rest = trees.as_slice();
        while let [first, after @ ..] = rest {
            rest = after;
            match first {
                TokenTree::Group(group) => self.tokens(group.stream(), literals),
                TokenTree::Ident(ident) => {
                    let mut segments = vec![name(ident)];
                    while let [
                        TokenTree::Punct(first),
                        TokenTree::Punct(second),
                        TokenTree::Ident(next),
                        after @ ..,
                    ] = rest
                        && first.as_char() == ':'
                        && second.as_char() == ':'
                    {
                        segments.push(name(next));
                        rest = after;
                    }
                    self.reference(segments, Site::Path, line(ident));
                }
                // `#[serde(with = "crate::app::format")]` names a path in a string.
                TokenTree::Literal(literal) if literals => {
                    let path = syn::parse_str::<syn::LitStr>(&literal.to_string())
                        .and_then(|string| string.parse_with(syn::Path::parse_mod_style));
                    if let Ok(path) = path {
                        self.reference(segments(&path), Site::Path, literal.span().start().line);
                    }
                }
                TokenTree::Punct(_) | TokenTree::Literal(_) => {}
            }
        }
    }
}

fn name(ident: &Ident) -> String {
    ident.unraw().to_string()
}

fn line(ident: &Ident) -> usize {
    ident.span().start().line
}

fn segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| name(&segment.ident))
        .collect()
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let relocation = node.attrs.iter().find(|attr| attr.path().is_ident("path"));
        if let Some(attribute) = relocation {
            self.relocated.get_or_insert(attribute.span().start().line);
        }
        let name = name(&node.ident);
        let children = self.children.join(&name);
        let outer = mem::replace(&mut self.children, children);
        self.module.push(name);
        self.modules.push(Declared {
            module: self.module.clone(),
            children: node.content.is_none().then(|| self.children.clone()),
        });
        visit::visit_item_mod(self, node);
        self.module.pop();
        self.children = outer;
    }

    fn visit_use_tree(&mut self, node: &'ast syn::UseTree) {
        self.imports(node, &mut Vec::new());
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        if let Some(first) = node.segments.first() {
            self.reference(segments(node), Site::Path, line(&first.ident));
        }
        visit::visit_path(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        self.tokens(node.tokens.clone(), false);
        visit::visit_macro(self, node);
    }

    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if let syn::Meta::List(list) = &node.meta {
            self.tokens(list.tokens.clone(), true);
        }
        visit::visit_attribute(self, node);
    }
}
