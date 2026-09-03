//! Attribute grammar for validation declarations.
//!
//! This module intentionally stays independent from `simple_impl_core` so the
//! attr kit does not need to know the final semantic model. It parses validation
//! attributes into small grammar objects that `simple_impl_core` can lower into
//! `ValidationRule`.

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, LitStr, Token};

use crate::{AttrBag, AttrExpected, AttrSchema};

/// One rule as written in a `#[requires(..)]`, `#[one_of(..)]` or `#[validate(..)]` attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedValidationRule {
    /// `#[requires("other")]` on `field`: `field` needs `other` set too.
    Requires {
        /// The field the attribute is on.
        field: String,
        /// The field it needs.
        required: String,
    },
    /// `#[invalid_without("other")]` on `field`.
    InvalidWithout {
        /// The field the attribute is on.
        field: String,
        /// The field it needs.
        required: String,
    },
    /// `#[only_pair_with("other")]` on `field`: `field` may only appear with `other`.
    OnlyPairWith {
        /// The field the attribute is on.
        field: String,
        /// Its only allowed companion.
        paired_with: String,
    },
    /// `#[conflicts_with("other")]` on `field`.
    ConflictsWith {
        /// The field the attribute is on.
        field: String,
        /// The field it excludes.
        conflicts_with: String,
    },
    /// `#[one_of("a", "b")]`: exactly one of the fields.
    OneOf {
        /// The fields in the set.
        fields: Vec<String>,
    },
    /// `#[at_least_one_of("a", "b")]`.
    AtLeastOneOf {
        /// The fields in the set.
        fields: Vec<String>,
    },
    /// `#[validate(with = "path::to::fn")]`: a hook the generated code calls.
    CustomFunction {
        /// The function path as written.
        function_path: String,
    },
}

/// The rules gathered from one item's attributes, in source order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedValidationSpec {
    rules: Vec<ParsedValidationRule>,
}

impl ParsedValidationSpec {
    /// No rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// A spec holding `rules`.
    pub fn from_rules(rules: impl IntoIterator<Item = ParsedValidationRule>) -> Self {
        Self {
            rules: rules.into_iter().collect(),
        }
    }

    /// The rules, in order.
    pub fn rules(&self) -> &[ParsedValidationRule] {
        &self.rules
    }

    /// Whether there are no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Add a rule.
    pub fn push(&mut self, rule: ParsedValidationRule) -> &mut Self {
        self.rules.push(rule);
        self
    }
}

/// Parse validation attrs that are valid at command/group scope.
///
/// Supported grammar in this pass:
///
/// ```ignore
/// #[validate(with = "validate_cp_preflight")]
/// #[one_of("json", "yaml")]
/// #[at_least_one_of("source", "destination")]
/// ```
pub fn parse_validation_attrs(attrs: &[Attribute]) -> syn::Result<ParsedValidationSpec> {
    let mut spec = ParsedValidationSpec::new();

    for attr in attrs {
        if attr.path().is_ident("validate") {
            parse_validate_attr(attr, &mut spec)?;
        } else if attr.path().is_ident("one_of") {
            spec.push(ParsedValidationRule::OneOf {
                fields: parse_lit_str_list(attr)?,
            });
        } else if attr.path().is_ident("at_least_one_of") {
            spec.push(ParsedValidationRule::AtLeastOneOf {
                fields: parse_lit_str_list(attr)?,
            });
        }
    }

    Ok(spec)
}

/// Parse validation attrs that are valid on an individual field.
///
/// Field-level dependency attrs implicitly use `field_name` as the left-hand
/// side of the rule.
///
/// ```ignore
/// #[invalid_without("force")]
/// #[only_pair_with("delete")]
/// #[conflicts_with("yaml")]
/// #[requires("profile")]
/// #[validate(with = "validate_target")]
/// ```
pub fn parse_field_validation_attrs(
    field_name: impl Into<String>,
    attrs: &[Attribute],
) -> syn::Result<ParsedValidationSpec> {
    let field_name = field_name.into();
    let mut spec = parse_validation_attrs(attrs)?;

    for attr in attrs {
        if attr.path().is_ident("requires") {
            spec.push(ParsedValidationRule::Requires {
                field: field_name.clone(),
                required: parse_single_lit_str(attr)?,
            });
        } else if attr.path().is_ident("invalid_without") {
            spec.push(ParsedValidationRule::InvalidWithout {
                field: field_name.clone(),
                required: parse_single_lit_str(attr)?,
            });
        } else if attr.path().is_ident("only_pair_with") {
            spec.push(ParsedValidationRule::OnlyPairWith {
                field: field_name.clone(),
                paired_with: parse_single_lit_str(attr)?,
            });
        } else if attr.path().is_ident("conflicts_with") {
            spec.push(ParsedValidationRule::ConflictsWith {
                field: field_name.clone(),
                conflicts_with: parse_single_lit_str(attr)?,
            });
        }
    }

    Ok(spec)
}

fn parse_validate_attr(attr: &Attribute, spec: &mut ParsedValidationSpec) -> syn::Result<()> {
    let bag = AttrBag::from_attr(attr, "validate")?;
    let bag = AttrSchema::new()
        .optional("with", AttrExpected::String)
        .validate(&bag)?;

    if let Some(function_path) = bag.optional_str("with")? {
        spec.push(ParsedValidationRule::CustomFunction {
            function_path: function_path.to_owned(),
        });
    }

    Ok(())
}

fn parse_single_lit_str(attr: &Attribute) -> syn::Result<String> {
    Ok(attr.parse_args::<LitStr>()?.value())
}

fn parse_lit_str_list(attr: &Attribute) -> syn::Result<Vec<String>> {
    Ok(attr.parse_args::<LitStrList>()?.values)
}

struct LitStrList {
    values: Vec<String>,
}

impl Parse for LitStrList {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let values = Punctuated::<LitStr, Token![,]>::parse_terminated(input)?
            .into_iter()
            .map(|value| value.value())
            .collect();

        Ok(Self { values })
    }
}
