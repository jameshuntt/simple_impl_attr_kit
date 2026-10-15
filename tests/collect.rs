//! `AttrSchema::validate` checks every key and reports every problem at once.

use simple_impl_attr_kit::{AttrBag, AttrExpected, AttrSchema, AttrValue};
use syn::parse::Parser;

fn schema(open: bool) -> AttrSchema {
    let s = AttrSchema::new()
        .required("name", AttrExpected::String)
        .optional("count", AttrExpected::Int)
        .with_default("mode", AttrExpected::String, AttrValue::from("a"))
        .one_of("mode", &["a", "b"])
        .repeatable("tag", AttrExpected::String)
        .optional("flag", AttrExpected::Bool)
        .optional("off", AttrExpected::Bool)
        .alias("nm", "name")
        .requires("count", "flag")
        .conflicts("flag", "off");
    if open { s.allow_unknown() } else { s }
}

#[test]
fn every_bad_key_is_reported_at_once() {
    let src = "#[x(nm = \"a\", name = \"b\", count = \"s\", mode = \"z\", tag = 1, bogus = 1, off)] #[x(flag)]";
    let attrs = syn::Attribute::parse_outer.parse_str(src).unwrap();
    let bag = AttrBag::from_attrs(&attrs, "x").unwrap();
    let all: Vec<String> = schema(false).validate(&bag).unwrap_err().into_iter().map(|e| e.to_string()).collect();
    assert_eq!(
        all,
        [
            "`nm` and `name` are the same key; give it once",
            "unknown attribute key `bogus` known keys: count, flag, mode, name, off, tag",
            "expected an integer for `count`, got string",
            "`mode` must be one of \"a\", \"b\", not \"z\"",
            "expected a string for each `tag`, got integer",
            "`flag` cannot be used with `off`",
        ]
    );
}

#[test]
fn a_misspelt_required_key_is_reported_once() {
    let attrs = syn::Attribute::parse_outer.parse_str("#[x(nmae = \"a\")]").unwrap();
    let bag = AttrBag::from_attrs(&attrs, "x").unwrap();
    let all: Vec<String> = schema(false).validate(&bag).unwrap_err().into_iter().map(|e| e.to_string()).collect();
    assert_eq!(all, ["unknown attribute key `nmae`; did you mean `name`? known keys: count, flag, mode, name, off, tag"]);
}

#[test]
fn a_misspelt_key_does_not_also_break_a_requires_rule() {
    let attrs = syn::Attribute::parse_outer.parse_str("#[x(name = \"a\", count = 1, flg)]").unwrap();
    let bag = AttrBag::from_attrs(&attrs, "x").unwrap();
    let all: Vec<String> = schema(false).validate(&bag).unwrap_err().into_iter().map(|e| e.to_string()).collect();
    assert_eq!(all, ["unknown attribute key `flg`; did you mean `flag`? known keys: count, flag, mode, name, off, tag"]);
}

#[test]
fn every_bad_validation_attribute_is_reported_and_the_good_rules_kept() {
    use simple_impl_attr_kit::{parse_field_validation_attrs_into, ParsedValidationRule};
    let attrs = syn::Attribute::parse_outer
        .parse_str("#[one_of(1)] #[at_least_one_of(\"a\", \"b\")] #[requires(5)] #[conflicts_with(\"c\")]")
        .unwrap();
    let mut errs = liaise_proc_macro::SynBuffer::new();
    let spec = parse_field_validation_attrs_into("f", &attrs, &mut errs);
    assert_eq!(errs.finish().unwrap_err().into_iter().count(), 2);
    assert_eq!(
        spec.rules(),
        [
            ParsedValidationRule::AtLeastOneOf { fields: vec!["a".into(), "b".into()] },
            ParsedValidationRule::ConflictsWith { field: "f".into(), conflicts_with: "c".into() },
        ]
    );
}
