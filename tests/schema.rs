//! The rules a schema can express, and the errors it gives.

use simple_impl_attr_kit::{AttrBag, AttrExpected, AttrSchema, AttrValue};
use syn::{parse_quote, Attribute};

fn bag(attr: Attribute) -> AttrBag {
    AttrBag::from_attr(&attr, "shell").unwrap()
}

fn schema() -> AttrSchema {
    AttrSchema::new()
        .required("cmd", AttrExpected::String)
        .optional("order", AttrExpected::Int)
        .with_default("sep", AttrExpected::String, AttrValue::from(" "))
        .repeatable("alias", AttrExpected::String)
        .one_of("style", &["dash", "equals"])
        .optional("first", AttrExpected::Bool)
        .optional("ty", AttrExpected::Tokens)
        .optional("env", AttrExpected::Nested)
        .optional("weight", AttrExpected::Float)
        .alias("separator", "sep")
        .requires("first", "order")
        .conflicts("style", "sep")
        .doc("cmd", "the program to run")
}

#[test]
fn a_valid_attribute_comes_back_with_defaults_and_lists_filled() {
    let v = schema().validate(&bag(parse_quote!(#[shell(cmd = "git", order = 2, first, ty = Vec<u8>, env(name = "X"), weight = 3)]))).unwrap();
    assert_eq!(v.require_str("cmd").unwrap(), "git");
    assert_eq!(v.require_str("sep").unwrap(), " ", "default filled in");
    assert_eq!(v.get_list_str("alias").unwrap(), Vec::<String>::new(), "repeatable absent is an empty list");
    assert_eq!(v.optional_float("weight").unwrap(), Some(3.0), "an integer satisfies Float");
    assert_eq!(v.optional_tokens("ty").unwrap(), Some("Vec < u8 >"));
    assert!(v.optional_nested("env").unwrap().is_some());
}

#[test]
fn repeatable_keys_collect_and_a_single_value_is_a_list_of_one() {
    let v = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", alias = "a", alias = "b")]))).unwrap();
    assert_eq!(v.get_list_str("alias").unwrap(), ["a", "b"]);
    let v = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", alias = "only")]))).unwrap();
    assert_eq!(v.get_list_str("alias").unwrap(), ["only"]);
    let v = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", alias("p", "q"))]))).unwrap();
    assert_eq!(v.get_list_str("alias").unwrap(), ["p", "q"]);
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", alias = 1)]))).unwrap_err();
    assert_eq!(err.to_string(), "expected a string for each `alias`, got integer");
}

#[test]
fn a_non_repeatable_key_given_twice_is_refused() {
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "a", cmd = "b")]))).unwrap_err();
    assert_eq!(err.to_string(), "`cmd` given 2 times; expected once");
}

#[test]
fn unknown_keys_are_refused_with_the_nearest_known_key() {
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", ordr = 1)]))).unwrap_err();
    assert_eq!(
        err.to_string(),
        "unknown attribute key `ordr`; did you mean `order`? known keys: alias, cmd, env, first, order, sep, style, ty, weight"
    );
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", zzzzzz = 1)]))).unwrap_err();
    assert!(err.to_string().starts_with("unknown attribute key `zzzzzz` known keys:"), "{err}");
    let v = schema().allow_unknown().validate(&bag(parse_quote!(#[shell(cmd = "g", extra = 1)]))).unwrap();
    assert_eq!(v.require_int("extra").unwrap(), 1);
}

#[test]
fn required_missing_and_wrong_kinds() {
    let err = schema().validate(&bag(parse_quote!(#[shell(order = 1)]))).unwrap_err();
    assert_eq!(err.to_string(), "missing required attribute key `cmd`");
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = 1)]))).unwrap_err();
    assert_eq!(err.to_string(), "expected a string for `cmd`, got integer");
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", order = "one")]))).unwrap_err();
    assert_eq!(err.to_string(), "expected an integer for `order`, got string");
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", env = 1)]))).unwrap_err();
    assert_eq!(err.to_string(), "expected a nested attribute for `env`, got integer");
}

#[test]
fn one_of_limits_the_choices() {
    assert!(schema().validate(&bag(parse_quote!(#[shell(cmd = "g", style = "dash")]))).is_ok());
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", style = "colon")]))).unwrap_err();
    assert_eq!(err.to_string(), "`style` must be one of \"dash\", \"equals\", not \"colon\"");
}

#[test]
fn aliases_resolve_and_may_not_be_doubled() {
    let v = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", separator = ",")]))).unwrap();
    assert_eq!(v.require_str("sep").unwrap(), ",");
    assert!(!v.contains_key("separator"));
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", separator = ",", sep = ";")]))).unwrap_err();
    assert_eq!(err.to_string(), "`separator` and `sep` are the same key; give it once");
}

#[test]
fn requires_and_conflicts_between_keys() {
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", first)]))).unwrap_err();
    assert_eq!(err.to_string(), "`first` requires `order`");
    assert!(schema().validate(&bag(parse_quote!(#[shell(cmd = "g", first, order = 1)]))).is_ok());
    let err = schema().validate(&bag(parse_quote!(#[shell(cmd = "g", style = "dash", sep = ",")]))).unwrap_err();
    assert_eq!(err.to_string(), "`style` cannot be used with `sep`");
}

#[test]
fn describe_lists_every_key_with_its_rules() {
    let text = schema().describe();
    assert!(text.contains("cmd: a string, required: the program to run\n"), "{text}");
    assert!(text.contains("alias: a string, repeatable\n"), "{text}");
    assert!(text.contains("sep: a string, default Str(\" \")\n"), "{text}");
    assert!(text.contains("style: a string, one of dash | equals\n"), "{text}");
    assert_eq!(schema().keys().count(), 9);
}

#[test]
fn a_validated_bag_keeps_the_key_spans() {
    let raw = bag(parse_quote!(#[shell(cmd = "g", order = 2)]));
    let v = schema().validate(&raw).unwrap();
    assert!(v.span_of("order").is_some());
    assert!(v.span_of("cmd").is_some());
    assert!(v.span_of("sep").is_some(), "a defaulted key gets the attribute's span");
}
