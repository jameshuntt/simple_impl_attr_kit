//! Parsing one attribute into keys and values, with a span per key.

use std::collections::BTreeMap;

use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::ToTokens;
use syn::parse::{ParseStream, Parser};
use syn::spanned::Spanned;
use syn::{Attribute, Ident, Lit, Token};

use crate::value::AttrValue;

/// One parsed key: its value and where the key was written.
///
/// Equality ignores the span: two bags parsed from the same text are equal.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The value.
    pub value: AttrValue,
    /// The span of the key, for errors that point at it.
    pub span: Span,
    /// Whether the key was written more than once; the value is then a list.
    pub repeated: bool,
}

/// The keys and values of one attribute, `#[name(...)]`, or of several
/// attributes with the same name merged.
///
/// ```
/// use simple_impl_attr_kit::{AttrBag, AttrValue};
/// use syn::parse_quote;
///
/// let attr: syn::Attribute = parse_quote!(#[shell(flag = "--force", order = 3, into, ty = Vec<u8>)]);
/// let bag = AttrBag::from_attr(&attr, "shell").unwrap();
/// assert_eq!(bag.require_str("flag").unwrap(), "--force");
/// assert_eq!(bag.require_int("order").unwrap(), 3);
/// assert!(bag.flag("into").unwrap());
/// assert_eq!(bag.optional_tokens("ty").unwrap(), Some("Vec < u8 >"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct AttrBag {
    entries: BTreeMap<String, Entry>,
    attr_span: Option<Span>,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.repeated == other.repeated
    }
}

impl PartialEq for AttrBag {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl AttrBag {
    /// An empty bag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse `#[expected_name(...)]`. Any other attribute is an error.
    pub fn from_attr(attr: &Attribute, expected_name: &str) -> syn::Result<Self> {
        if !attr.path().is_ident(expected_name) {
            return Err(syn::Error::new_spanned(attr, format!("expected #[{expected_name}(...)] attribute")));
        }
        let mut bag = Self { entries: BTreeMap::new(), attr_span: Some(attr.span()) };
        match &attr.meta {
            syn::Meta::Path(_) => Ok(bag),
            syn::Meta::List(list) => {
                let entries = parse_entries.parse2(list.tokens.clone())?;
                for (key, span, value) in entries {
                    bag.push(key, span, value);
                }
                Ok(bag)
            }
            syn::Meta::NameValue(nv) => Err(syn::Error::new_spanned(
                nv,
                format!("expected #[{expected_name}(key = value, ...)], not #[{expected_name} = ...]"),
            )),
        }
    }

    /// Parse and merge every `#[expected_name(...)]` in `attrs`; others are skipped.
    pub fn from_attrs(attrs: &[Attribute], expected_name: &str) -> syn::Result<Self> {
        let mut merged = Self::new();
        for attr in attrs {
            if !attr.path().is_ident(expected_name) {
                continue;
            }
            let bag = Self::from_attr(attr, expected_name)?;
            merged.merge(bag, attr)?;
        }
        Ok(merged)
    }

    /// Merge another bag in. A key present in both becomes a list of both
    /// values, in order, like a key written twice in one attribute.
    pub fn merge(&mut self, other: Self, _span_tokens: impl ToTokens) -> syn::Result<()> {
        if self.attr_span.is_none() {
            self.attr_span = other.attr_span;
        }
        for (key, entry) in other.entries {
            match entry.value {
                AttrValue::List(items) if entry.repeated => {
                    for item in items {
                        self.push(key.clone(), entry.span, item);
                    }
                }
                value => self.push(key, entry.span, value),
            }
        }
        Ok(())
    }

    /// Insert with a known key span (used when a validated bag is built from a parsed one).
    pub(crate) fn insert_spanned(&mut self, key: impl Into<String>, value: AttrValue, span: Span) {
        self.entries.insert(key.into(), Entry { value, span, repeated: false });
    }

    /// Insert or replace a value (no repetition semantics).
    pub fn insert(&mut self, key: impl Into<String>, value: AttrValue) -> Option<AttrValue> {
        let key = key.into();
        let span = self.entries.get(&key).map(|e| e.span).unwrap_or_else(Span::call_site);
        self.entries.insert(key, Entry { value, span, repeated: false }).map(|e| e.value)
    }

    fn push(&mut self, key: String, span: Span, value: AttrValue) {
        match self.entries.get_mut(&key) {
            None => {
                self.entries.insert(key, Entry { value, span, repeated: false });
            }
            Some(existing) if existing.repeated => {
                if let AttrValue::List(items) = &mut existing.value {
                    items.push(value);
                }
            }
            Some(existing) => {
                let first = std::mem::replace(&mut existing.value, AttrValue::Bool(false));
                existing.value = AttrValue::List(vec![first, value]);
                existing.repeated = true;
            }
        }
    }

    /// The value under `key`.
    pub fn get(&self, key: &str) -> Option<&AttrValue> {
        self.entries.get(key).map(|e| &e.value)
    }

    /// Where `key` was written, if it was.
    pub fn span_of(&self, key: &str) -> Option<Span> {
        self.entries.get(key).map(|e| e.span)
    }

    /// The span of the whole attribute, if this bag came from one.
    pub fn attr_span(&self) -> Span {
        self.attr_span.unwrap_or_else(Span::call_site)
    }

    /// Whether `key` is present.
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// The keys, sorted.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Every key with its value, sorted by key.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &AttrValue)> {
        self.entries.iter().map(|(k, e)| (k, &e.value))
    }

    /// Whether the bag has no keys.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many keys the bag has.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `key = "…"`, required.
    pub fn require_str(&self, key: &str) -> syn::Result<&str> {
        self.required(key, "string", AttrValue::as_str)
    }

    /// `key = "…"`, optional.
    pub fn optional_str(&self, key: &str) -> syn::Result<Option<&str>> {
        self.optional(key, "string", AttrValue::as_str)
    }

    /// `key` or `key = bool`, required.
    pub fn require_bool(&self, key: &str) -> syn::Result<bool> {
        self.required(key, "bool", AttrValue::as_bool)
    }

    /// `key` or `key = bool`, optional.
    pub fn optional_bool(&self, key: &str) -> syn::Result<Option<bool>> {
        self.optional(key, "bool", AttrValue::as_bool)
    }

    /// Whether `key` is set: absent is `false`, present without a value is `true`.
    pub fn flag(&self, key: &str) -> syn::Result<bool> {
        Ok(self.optional_bool(key)?.unwrap_or(false))
    }

    /// `key = 3`, required.
    pub fn require_int(&self, key: &str) -> syn::Result<i64> {
        self.required(key, "integer", AttrValue::as_int)
    }

    /// `key = 3`, optional.
    pub fn optional_int(&self, key: &str) -> syn::Result<Option<i64>> {
        self.optional(key, "integer", AttrValue::as_int)
    }

    /// `key = 1.5` (or an integer, widened), optional.
    pub fn optional_float(&self, key: &str) -> syn::Result<Option<f64>> {
        self.optional(key, "number", AttrValue::as_float)
    }

    /// `key = 'c'`, optional.
    pub fn optional_char(&self, key: &str) -> syn::Result<Option<char>> {
        self.optional(key, "char", AttrValue::as_char)
    }

    /// `key = <tokens>`, required.
    pub fn require_tokens(&self, key: &str) -> syn::Result<&str> {
        self.required(key, "tokens", AttrValue::as_tokens)
    }

    /// `key = <tokens>`, optional.
    pub fn optional_tokens(&self, key: &str) -> syn::Result<Option<&str>> {
        self.optional(key, "tokens", AttrValue::as_tokens)
    }

    /// `key(...)` / `key = [...]` / a repeated key, required.
    pub fn require_list(&self, key: &str) -> syn::Result<&[AttrValue]> {
        self.required(key, "list", AttrValue::as_list)
    }

    /// `key(...)` / `key = [...]` / a repeated key, optional.
    pub fn optional_list(&self, key: &str) -> syn::Result<Option<&[AttrValue]>> {
        self.optional(key, "list", AttrValue::as_list)
    }

    /// `key(inner = ..., ...)`, optional.
    pub fn optional_nested(&self, key: &str) -> syn::Result<Option<&AttrBag>> {
        self.optional(key, "nested attribute", AttrValue::as_nested)
    }

    /// The values of `key` as strings: a single string counts as a list of one,
    /// a list must be all strings. Absent is an error.
    pub fn get_list_str(&self, key: &str) -> syn::Result<Vec<String>> {
        let value = self.get(key).ok_or_else(|| self.expected_error(key, "list of strings"))?;
        match value {
            AttrValue::Str(s) => Ok(vec![s.clone()]),
            AttrValue::List(items) => items
                .iter()
                .map(|v| v.as_str().map(ToOwned::to_owned).ok_or_else(|| self.expected_error(key, "list of strings")))
                .collect(),
            _ => Err(self.expected_error(key, "list of strings")),
        }
    }

    /// Like [`get_list_str`](Self::get_list_str) but absent is an empty list.
    pub fn list_str_or_empty(&self, key: &str) -> syn::Result<Vec<String>> {
        if self.contains_key(key) {
            self.get_list_str(key)
        } else {
            Ok(Vec::new())
        }
    }

    fn required<'a, T>(&'a self, key: &str, expected: &str, read: fn(&'a AttrValue) -> Option<T>) -> syn::Result<T> {
        match self.get(key) {
            Some(value) => read(value).ok_or_else(|| self.mismatch_error(key, expected, value)),
            None => Err(syn::Error::new(self.attr_span(), format!("missing required attribute key `{key}` ({expected})"))),
        }
    }

    fn optional<'a, T>(
        &'a self,
        key: &str,
        expected: &str,
        read: fn(&'a AttrValue) -> Option<T>,
    ) -> syn::Result<Option<T>> {
        match self.get(key) {
            Some(value) => read(value).map(Some).ok_or_else(|| self.mismatch_error(key, expected, value)),
            None => Ok(None),
        }
    }

    fn mismatch_error(&self, key: &str, expected: &str, value: &AttrValue) -> syn::Error {
        let got = match value {
            AttrValue::List(items) if self.entries.get(key).map(existing_was_repeated).unwrap_or(false) => {
                format!("`{key}` given {} times; expected once", items.len())
            }
            other => format!("expected {expected} for `{key}`, got {}", other.kind_name()),
        };
        syn::Error::new(self.span_of(key).unwrap_or_else(|| self.attr_span()), got)
    }

    pub(crate) fn expected_error(&self, key: &str, expected: &str) -> syn::Error {
        match self.get(key) {
            Some(value) => self.mismatch_error(key, expected, value),
            None => syn::Error::new(self.attr_span(), format!("missing required attribute key `{key}` ({expected})")),
        }
    }

    pub(crate) fn was_repeated(&self, key: &str) -> bool {
        self.entries.get(key).map(existing_was_repeated).unwrap_or(false)
    }
}

fn existing_was_repeated(entry: &Entry) -> bool {
    entry.repeated
}

/// `key`, `key = value`, `key(...)`, comma separated, trailing comma allowed.
fn parse_entries(input: ParseStream<'_>) -> syn::Result<Vec<(String, Span, AttrValue)>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let (key, span) = parse_key(input)?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            parse_value(input)?
        } else if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            parse_group(&content, &key, span)?
        } else {
            AttrValue::Bool(true)
        };
        out.push((key, span, value));
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(out)
}

/// A key is an identifier, a keyword (`type`, `async`), a raw identifier
/// (read without its `r#`), or a `::`-joined path of those.
fn parse_key(input: ParseStream<'_>) -> syn::Result<(String, Span)> {
    use syn::ext::IdentExt;
    let first: Ident = input
        .call(Ident::parse_any)
        .map_err(|e| syn::Error::new(e.span(), "expected an attribute key (an identifier such as `flag` or `order`)"))?;
    let span = first.span();
    let mut key = first.unraw().to_string();
    while input.peek(Token![::]) {
        input.parse::<Token![::]>()?;
        let next: Ident = input.call(Ident::parse_any)?;
        key.push_str("::");
        key.push_str(&next.unraw().to_string());
    }
    Ok((key, span))
}

/// The value after `=`: a literal (with an optional leading minus), a
/// bracketed list, or verbatim tokens up to the next top-level comma.
fn parse_value(input: ParseStream<'_>) -> syn::Result<AttrValue> {
    if input.peek(syn::token::Bracket) {
        let content;
        syn::bracketed!(content in input);
        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(parse_value(&content)?);
            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }
        return Ok(AttrValue::List(items));
    }
    let negative = input.peek(Token![-]) && (input.peek2(syn::LitInt) || input.peek2(syn::LitFloat));
    if negative {
        input.parse::<Token![-]>()?;
    }
    if input.peek(Lit) && !input.peek2(Token![.]) && !input.peek2(Token![::]) {
        let lit: Lit = input.parse()?;
        let value = match lit {
            Lit::Str(s) => AttrValue::Str(s.value()),
            Lit::Bool(b) => AttrValue::Bool(b.value()),
            Lit::Int(i) => {
                let v = i.base10_parse::<i64>()?;
                AttrValue::Int(if negative { -v } else { v })
            }
            Lit::Float(f) => {
                let v = f.base10_parse::<f64>()?;
                AttrValue::Float(if negative { -v } else { v })
            }
            Lit::Char(c) => AttrValue::Char(c.value()),
            Lit::ByteStr(b) => AttrValue::Bytes(b.value()),
            other => AttrValue::Tokens(other.to_token_stream().to_string()),
        };
        return Ok(value);
    }
    let mut tokens = TokenStream2::new();
    if negative {
        tokens.extend(std::iter::once(TokenTree::Punct(proc_macro2::Punct::new('-', proc_macro2::Spacing::Alone))));
    }
    let mut angle_depth = 0usize;
    while !input.is_empty() {
        if angle_depth == 0 && input.peek(Token![,]) {
            break;
        }
        let token = input.parse::<TokenTree>()?;
        match &token {
            TokenTree::Punct(p) if p.as_char() == '<' => angle_depth = angle_depth.saturating_add(1),
            TokenTree::Punct(p) if p.as_char() == '>' => angle_depth = angle_depth.saturating_sub(1),
            _ => {}
        }
        tokens.extend(std::iter::once(token));
    }
    if tokens.is_empty() {
        return Err(input.error("expected a value after `=`"));
    }
    Ok(AttrValue::Tokens(tokens.to_string()))
}

/// The inside of `key(...)`: a list of values, or a nested bag of keys.
fn parse_group(content: ParseStream<'_>, key: &str, span: Span) -> syn::Result<AttrValue> {
    if content.is_empty() {
        return Ok(AttrValue::List(Vec::new()));
    }
    let nested = looks_like_key(content);
    if nested {
        let entries = parse_entries(content)?;
        let mut bag = AttrBag { entries: BTreeMap::new(), attr_span: Some(span) };
        for (k, s, v) in entries {
            bag.push(k, s, v);
        }
        Ok(AttrValue::Nested(bag))
    } else {
        let mut items = Vec::new();
        while !content.is_empty() {
            if looks_like_key(content) {
                return Err(syn::Error::new(
                    content.span(),
                    format!("`{key}(...)` mixes plain values with `key = value` entries; use one or the other"),
                ));
            }
            items.push(parse_value(content)?);
            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }
        Ok(AttrValue::List(items))
    }
}

/// `ident`, `ident = …`, `ident(…)` or a `::` path at the front: a key, not a value.
fn looks_like_key(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    if fork.parse::<Ident>().is_err() && fork.parse::<syn::Path>().is_err() {
        return false;
    }
    fork.is_empty() || fork.peek(Token![=]) || fork.peek(Token![,]) || fork.peek(syn::token::Paren)
}
