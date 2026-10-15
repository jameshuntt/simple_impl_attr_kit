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

#[test]
fn a_value_keeps_its_tokens_as_written_through_merge_and_validate() {
    let attrs = syn::Attribute::parse_outer
        .parse_str("#[x(e = self.n + 1, s = \"text\", flag)] #[x(nm = \"a\", l(1, 2))]")
        .unwrap();
    let bag = AttrBag::from_attrs(&attrs, "x").unwrap();
    let written = |bag: &AttrBag, key: &str| bag.written_tokens(key).map(ToString::to_string);
    assert_eq!(written(&bag, "e").as_deref(), Some("self . n + 1"));
    assert_eq!(written(&bag, "s").as_deref(), Some("\"text\""));
    assert_eq!(written(&bag, "flag"), None);
    assert_eq!(written(&bag, "l"), None);

    let schema = AttrSchema::new()
        .required("name", AttrExpected::String)
        .optional("e", AttrExpected::Any)
        .optional("s", AttrExpected::String)
        .optional("flag", AttrExpected::Bool)
        .optional("l", AttrExpected::List)
        .alias("nm", "name");
    let valid = schema.validate(&bag).unwrap();
    assert_eq!(written(&valid, "e").as_deref(), Some("self . n + 1"));
    // an alias's tokens follow it to the key it stands for
    assert_eq!(written(&valid, "name").as_deref(), Some("\"a\""));
}

#[test]
fn a_value_that_is_code_reads_whole_up_to_the_next_key() {
    let attrs = syn::Attribute::parse_outer
        .parse_str(
            "#[x(a = 2 * self.n, b = self.a < self.b, c = [0u8; 4], d = self.n << 1, e = -1 * self.n, \
             t = HashMap<String, u8>, l = [1, 2], n = -3, s = \"str\", m = 1)]",
        )
        .unwrap();
    let bag = AttrBag::from_attrs(&attrs, "x").unwrap();
    let tokens = |k: &str| bag.get(k).and_then(|v| v.as_tokens()).map(ToString::to_string);
    assert_eq!(tokens("a").as_deref(), Some("2 * self . n"));
    assert_eq!(tokens("b").as_deref(), Some("self . a < self . b"));
    // `[0u8; 4]` reads as a one-item list (it was an error before); as written it is the whole array
    assert_eq!(bag.written_tokens("c").map(ToString::to_string).as_deref(), Some("[0u8 ; 4]"));
    assert_eq!(tokens("d").as_deref(), Some("self . n << 1"));
    assert_eq!(tokens("e").as_deref(), Some("- 1 * self . n"));
    // what was a value before still is, unchanged
    assert_eq!(tokens("t").as_deref(), Some("HashMap < String , u8 >"));
    assert_eq!(bag.get("l"), Some(&AttrValue::List(vec![AttrValue::Int(1), AttrValue::Int(2)])));
    assert_eq!(bag.get("n"), Some(&AttrValue::Int(-3)));
    assert_eq!(bag.get("s"), Some(&AttrValue::Str("str".into())));
    assert_eq!(bag.get("m"), Some(&AttrValue::Int(1)));
    assert_eq!(bag.written_tokens("b").map(ToString::to_string).as_deref(), Some("self . a < self . b"));
}

#[test]
fn a_value_inside_an_invisible_group_parses_as_before_and_is_simply_not_kept() {
    use proc_macro2::{Delimiter, Group, TokenStream};
    use quote::quote;
    // what a macro_rules `$e:expr` hands over: the value inside an invisible group
    let one_comma = Group::new(Delimiter::None, quote!(1 ,));
    let attr: syn::Attribute = syn::parse_quote!(#[x(k = #one_comma m = 2)]);
    let bag = AttrBag::from_attr(&attr, "x").unwrap();
    assert_eq!(bag.get("k"), Some(&AttrValue::Int(1)));
    assert_eq!(bag.get("m"), Some(&AttrValue::Int(2)));
    assert_eq!(bag.written_tokens("k").map(TokenStream::to_string), None);
    let whole = Group::new(Delimiter::None, quote!(1 + 1));
    let attr: syn::Attribute = syn::parse_quote!(#[x(k = #whole, m = 2)]);
    let bag = AttrBag::from_attr(&attr, "x").unwrap();
    assert!(bag.written_tokens("k").is_some());
    assert_eq!(bag.get("m"), Some(&AttrValue::Int(2)));
}
