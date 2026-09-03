//! `///` comments as a string.

use syn::{Attribute, Expr, ExprLit, Lit, Meta};

/// The `///` comments on an item, trimmed and joined with newlines; `None` when there are none.
pub fn extract_doc_comments(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }

        let Meta::NameValue(name_value) = &attr.meta else {
            continue;
        };

        let Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) = &name_value.value
        else {
            continue;
        };

        docs.push(value.value().trim().to_owned());
    }

    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}
