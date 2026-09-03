//! What an attribute may say: the keys, their kinds, and the rules between them.

use std::collections::{BTreeMap, BTreeSet};

use crate::{AttrBag, AttrValue};

/// The kind a key must hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrExpected {
    /// Anything.
    Any,
    /// `key = "…"`
    String,
    /// `key` or `key = bool`
    Bool,
    /// `key = 3`
    Int,
    /// `key = 1.5` or an integer
    Float,
    /// `key = 'c'`
    Char,
    /// `key = <tokens>`: a type, path or expression
    Tokens,
    /// `key(...)`, `key = [...]`, or a repeated key
    List,
    /// `key(inner = ...)`
    Nested,
}

impl AttrExpected {
    fn matches(self, value: &AttrValue) -> bool {
        match self {
            Self::Any => true,
            Self::String => matches!(value, AttrValue::Str(_)),
            Self::Bool => matches!(value, AttrValue::Bool(_)),
            Self::Int => matches!(value, AttrValue::Int(_)),
            Self::Float => matches!(value, AttrValue::Float(_) | AttrValue::Int(_)),
            Self::Char => matches!(value, AttrValue::Char(_)),
            Self::Tokens => matches!(value, AttrValue::Tokens(_)),
            Self::List => matches!(value, AttrValue::List(_)),
            Self::Nested => matches!(value, AttrValue::Nested(_)),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Any => "any value",
            Self::String => "a string",
            Self::Bool => "a bool",
            Self::Int => "an integer",
            Self::Float => "a number",
            Self::Char => "a char",
            Self::Tokens => "tokens (a type, path or expression)",
            Self::List => "a list",
            Self::Nested => "a nested attribute",
        }
    }
}

#[derive(Debug, Clone)]
struct Rule {
    required: bool,
    expected: AttrExpected,
    default: Option<AttrValue>,
    repeatable: bool,
    one_of: Option<Vec<String>>,
    doc: Option<String>,
}

/// The rules for one attribute name.
///
/// ```
/// use simple_impl_attr_kit::{AttrBag, AttrExpected, AttrSchema, AttrValue};
/// use syn::parse_quote;
///
/// let schema = AttrSchema::new()
///     .required("cmd", AttrExpected::String)
///     .optional("order", AttrExpected::Int)
///     .with_default("sep", AttrExpected::String, AttrValue::from(" "))
///     .repeatable("alias", AttrExpected::String)
///     .one_of("style", &["dash", "equals"])
///     .conflicts("order", "first")
///     .optional("first", AttrExpected::Bool);
///
/// let attr: syn::Attribute = parse_quote!(#[shell(cmd = "git", alias = "g", alias = "gt", style = "dash")]);
/// let bag = schema.validate(&AttrBag::from_attr(&attr, "shell").unwrap()).unwrap();
/// assert_eq!(bag.require_str("sep").unwrap(), " ");
/// assert_eq!(bag.get_list_str("alias").unwrap(), ["g", "gt"]);
///
/// let attr: syn::Attribute = parse_quote!(#[shell(cmd = "git", styl = "dash")]);
/// let err = schema.validate(&AttrBag::from_attr(&attr, "shell").unwrap()).unwrap_err();
/// assert_eq!(err.to_string(), "unknown attribute key `styl`; did you mean `style`? known keys: alias, cmd, first, order, sep, style");
/// ```
#[derive(Debug, Clone, Default)]
pub struct AttrSchema {
    fields: BTreeMap<String, Rule>,
    aliases: BTreeMap<String, String>,
    requires: Vec<(String, String)>,
    conflicts: Vec<(String, String)>,
    allow_unknown: bool,
}

impl AttrSchema {
    /// No keys yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep keys the schema does not name instead of refusing them.
    pub fn allow_unknown(mut self) -> Self {
        self.allow_unknown = true;
        self
    }

    /// A key that must be present.
    pub fn required(mut self, name: &str, expected: AttrExpected) -> Self {
        self.fields.insert(name.to_owned(), Rule { required: true, expected, default: None, repeatable: false, one_of: None, doc: None });
        self
    }

    /// A key that may be present.
    pub fn optional(mut self, name: &str, expected: AttrExpected) -> Self {
        self.fields.insert(name.to_owned(), Rule { required: false, expected, default: None, repeatable: false, one_of: None, doc: None });
        self
    }

    /// A key that may be present and has a value when it is not.
    pub fn with_default(mut self, name: &str, expected: AttrExpected, default: AttrValue) -> Self {
        self.fields.insert(name.to_owned(), Rule { required: false, expected, default: Some(default), repeatable: false, one_of: None, doc: None });
        self
    }

    /// A key that may be written several times; the validated value is
    /// always a list of `expected`, empty when absent.
    pub fn repeatable(mut self, name: &str, expected: AttrExpected) -> Self {
        self.fields.insert(name.to_owned(), Rule { required: false, expected, default: None, repeatable: true, one_of: None, doc: None });
        self
    }

    /// A string key whose value must be one of `choices`.
    pub fn one_of(mut self, name: &str, choices: &[&str]) -> Self {
        let rule = self.fields.entry(name.to_owned()).or_insert(Rule {
            required: false,
            expected: AttrExpected::String,
            default: None,
            repeatable: false,
            one_of: None,
            doc: None,
        });
        rule.expected = AttrExpected::String;
        rule.one_of = Some(choices.iter().map(|c| (*c).to_owned()).collect());
        self
    }

    /// An old name for a key: `old` is accepted and read as `new`.
    pub fn alias(mut self, old: &str, new: &str) -> Self {
        self.aliases.insert(old.to_owned(), new.to_owned());
        self
    }

    /// `key` may only appear together with `other`.
    pub fn requires(mut self, key: &str, other: &str) -> Self {
        self.requires.push((key.to_owned(), other.to_owned()));
        self
    }

    /// `key` and `other` may not appear together.
    pub fn conflicts(mut self, key: &str, other: &str) -> Self {
        self.conflicts.push((key.to_owned(), other.to_owned()));
        self
    }

    /// A line of documentation for a key, shown by [`describe`](Self::describe).
    pub fn doc(mut self, name: &str, text: &str) -> Self {
        if let Some(rule) = self.fields.get_mut(name) {
            rule.doc = Some(text.to_owned());
        }
        self
    }

    /// The keys this schema knows, sorted.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.fields.keys()
    }

    /// One line per key: name, kind, required or default, choices, doc.
    /// For error messages and generated documentation.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        for (name, rule) in &self.fields {
            let mut line = format!("{name}: {}", rule.expected.name());
            if rule.required {
                line.push_str(", required");
            }
            if rule.repeatable {
                line.push_str(", repeatable");
            }
            if let Some(default) = &rule.default {
                line.push_str(&format!(", default {default:?}"));
            }
            if let Some(choices) = &rule.one_of {
                line.push_str(&format!(", one of {}", choices.join(" | ")));
            }
            if let Some(doc) = &rule.doc {
                line.push_str(&format!(": {doc}"));
            }
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    /// Check `parsed` against the rules and return the validated bag: aliases
    /// resolved, defaults filled, repeatable keys as lists. Errors point at
    /// the key they are about.
    pub fn validate(&self, parsed: &AttrBag) -> syn::Result<AttrBag> {
        // resolve aliases into a working copy
        let mut working = AttrBag::new();
        let mut spans: BTreeMap<String, proc_macro2::Span> = BTreeMap::new();
        for (key, value) in parsed.entries() {
            let name = self.aliases.get(key).cloned().unwrap_or_else(|| key.clone());
            let span = parsed.span_of(key).unwrap_or_else(|| parsed.attr_span());
            if working.contains_key(&name) {
                return Err(syn::Error::new(span, format!("`{key}` and `{name}` are the same key; give it once")));
            }
            working.insert(name.clone(), value.clone());
            spans.insert(name, span);
        }
        let span_of = |key: &str| spans.get(key).copied().unwrap_or_else(|| parsed.attr_span());

        if !self.allow_unknown {
            let known: BTreeSet<&String> = self.fields.keys().collect();
            for key in working.keys() {
                if !known.contains(key) {
                    let mut msg = format!("unknown attribute key `{key}`");
                    if let Some(near) = closest(key, self.fields.keys()) {
                        msg.push_str(&format!("; did you mean `{near}`?"));
                    }
                    let list: Vec<&str> = self.fields.keys().map(String::as_str).collect();
                    msg.push_str(&format!(" known keys: {}", list.join(", ")));
                    return Err(syn::Error::new(span_of(key), msg));
                }
            }
        }

        let mut result = AttrBag::new();
        for (name, rule) in &self.fields {
            match working.get(name) {
                Some(value) if rule.repeatable => {
                    let items: Vec<AttrValue> = match value {
                        AttrValue::List(items) if parsed.was_repeated(name) || matches!(rule.expected, AttrExpected::List) => items.clone(),
                        AttrValue::List(items) => items.clone(),
                        single => vec![single.clone()],
                    };
                    for item in &items {
                        if !rule.expected.matches(item) {
                            return Err(syn::Error::new(
                                span_of(name),
                                format!("expected {} for each `{name}`, got {}", rule.expected.name(), item.kind_name()),
                            ));
                        }
                    }
                    result.insert_spanned(name.clone(), AttrValue::List(items), span_of(name));
                }
                Some(AttrValue::List(items)) if parsed.was_repeated(name) => {
                    return Err(syn::Error::new(span_of(name), format!("`{name}` given {} times; expected once", items.len())));
                }
                Some(value) if rule.expected.matches(value) => {
                    if let (Some(choices), Some(s)) = (&rule.one_of, value.as_str()) {
                        if !choices.iter().any(|c| c == s) {
                            return Err(syn::Error::new(
                                span_of(name),
                                format!("`{name}` must be one of {}, not \"{s}\"", choices.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(", ")),
                            ));
                        }
                    }
                    result.insert_spanned(name.clone(), value.clone(), span_of(name));
                }
                Some(value) => {
                    return Err(syn::Error::new(
                        span_of(name),
                        format!("expected {} for `{name}`, got {}", rule.expected.name(), value.kind_name()),
                    ));
                }
                None if rule.required => {
                    return Err(syn::Error::new(parsed.attr_span(), format!("missing required attribute key `{name}`")));
                }
                None if rule.repeatable => {
                    result.insert_spanned(name.clone(), AttrValue::List(Vec::new()), parsed.attr_span());
                }
                None => {
                    if let Some(default) = &rule.default {
                        result.insert_spanned(name.clone(), default.clone(), parsed.attr_span());
                    }
                }
            }
        }

        for (key, other) in &self.requires {
            if working.contains_key(key) && !working.contains_key(other) {
                return Err(syn::Error::new(span_of(key), format!("`{key}` requires `{other}`")));
            }
        }
        for (key, other) in &self.conflicts {
            if working.contains_key(key) && working.contains_key(other) {
                return Err(syn::Error::new(span_of(key), format!("`{key}` cannot be used with `{other}`")));
            }
        }

        if self.allow_unknown {
            for (key, value) in working.entries() {
                if !result.contains_key(key) {
                    result.insert_spanned(key.clone(), value.clone(), span_of(key));
                }
            }
        }
        Ok(result)
    }
}

/// The known key closest to `key`, when it is close enough to be a typo.
fn closest<'a>(key: &str, known: impl Iterator<Item = &'a String>) -> Option<&'a String> {
    known
        .map(|k| (edit_distance(key, k), k))
        .filter(|(d, k)| *d <= 2 || (*d <= k.len() / 3 && *d < key.len()))
        .min_by_key(|(d, _)| *d)
        .map(|(_, k)| k)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}
