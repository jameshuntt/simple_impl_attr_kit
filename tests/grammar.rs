//! Every shape a key can take, and what each one parses to.

use simple_impl_attr_kit::{AttrBag, AttrValue};
use syn::{parse_quote, Attribute};

fn bag(attr: Attribute, name: &str) -> AttrBag {
    AttrBag::from_attr(&attr, name).unwrap()
}

#[test]
fn a_bare_key_is_true() {
    let b = bag(parse_quote!(#[x(flag)]), "x");
    assert_eq!(b.get("flag"), Some(&AttrValue::Bool(true)));
    assert!(b.flag("flag").unwrap());
    assert!(!b.flag("absent").unwrap());
    assert_eq!(b.optional_bool("absent").unwrap(), None);
}

#[test]
fn every_literal_kind() {
    let b = bag(
        parse_quote!(#[x(s = "text", t = true, f = false, i = 42, n = -7, r = 1.5, m = -0.25, c = 'q', by = b"raw")]),
        "x",
    );
    assert_eq!(b.require_str("s").unwrap(), "text");
    assert!(b.require_bool("t").unwrap());
    assert!(!b.require_bool("f").unwrap());
    assert_eq!(b.require_int("i").unwrap(), 42);
    assert_eq!(b.require_int("n").unwrap(), -7);
    assert_eq!(b.optional_float("r").unwrap(), Some(1.5));
    assert_eq!(b.optional_float("m").unwrap(), Some(-0.25));
    assert_eq!(b.optional_float("i").unwrap(), Some(42.0), "an integer widens to a number");
    assert_eq!(b.optional_char("c").unwrap(), Some('q'));
    assert_eq!(b.get("by").unwrap().as_bytes(), Some(&b"raw"[..]));
    assert_eq!(b.len(), 9);
}

#[test]
fn types_paths_and_expressions_are_kept_as_tokens() {
    let b = bag(
        parse_quote!(#[x(ty = Vec<Option<u8>>, path = std::collections::HashMap<String, Vec<u8>>, call = f(1, 2), neg = -x, closure = |a| a + 1)]),
        "x",
    );
    // token text keeps the tokens, not the spacing; compare without spaces
    let compact = |s: &str| s.replace(' ', "");
    assert_eq!(compact(b.require_tokens("ty").unwrap()), "Vec<Option<u8>>");
    assert_eq!(compact(b.optional_tokens("path").unwrap().unwrap()), "std::collections::HashMap<String,Vec<u8>>");
    assert_eq!(compact(b.optional_tokens("call").unwrap().unwrap()), "f(1,2)", "the comma inside the call does not end the value");
    assert_eq!(compact(b.optional_tokens("neg").unwrap().unwrap()), "-x");
    assert_eq!(compact(b.optional_tokens("closure").unwrap().unwrap()), "|a|a+1");
    assert!(b.require_str("ty").is_err(), "tokens are not a string");
}

#[test]
fn a_path_with_a_dot_or_double_colon_after_a_literal_is_still_tokens() {
    let b = bag(parse_quote!(#[x(a = 1.max(2), b = "s".len())]), "x");
    let compact = |s: &str| s.replace(' ', "");
    assert_eq!(compact(b.optional_tokens("a").unwrap().unwrap()), "1.max(2)");
    assert_eq!(compact(b.optional_tokens("b").unwrap().unwrap()), "\"s\".len()");
}

#[test]
fn lists_in_parentheses_and_brackets() {
    let b = bag(parse_quote!(#[x(p("a", "b", 3), q = ["c", 'd', 2.5], empty(), mixed(x::Y, "s", -1))]), "x");
    assert_eq!(b.get_list_str("p").unwrap_err().to_string(), "expected list of strings for `p`, got list");
    let p = b.require_list("p").unwrap();
    assert_eq!(p, [AttrValue::from("a"), AttrValue::from("b"), AttrValue::Int(3)]);
    let q = b.require_list("q").unwrap();
    assert_eq!(q, [AttrValue::from("c"), AttrValue::Char('d'), AttrValue::Float(2.5)]);
    assert_eq!(b.require_list("empty").unwrap(), []);
    let mixed = b.require_list("mixed").unwrap();
    assert_eq!(mixed[0], AttrValue::Tokens("x :: Y".into()));
    assert_eq!(mixed[2], AttrValue::Int(-1));
    assert_eq!(b.optional_list("absent").unwrap(), None);
}

#[test]
fn a_list_of_strings_reads_as_strings_and_a_single_string_counts_as_one() {
    let b = bag(parse_quote!(#[x(many("a", "b"), one = "c", bracket = ["d"])]), "x");
    assert_eq!(b.get_list_str("many").unwrap(), ["a", "b"]);
    assert_eq!(b.get_list_str("one").unwrap(), ["c"]);
    assert_eq!(b.get_list_str("bracket").unwrap(), ["d"]);
    assert_eq!(b.list_str_or_empty("absent").unwrap(), Vec::<String>::new());
    assert!(b.get_list_str("absent").is_err());
}

#[test]
fn nested_keys() {
    let b = bag(parse_quote!(#[x(env(name = "HOME", required, retries = 3), deep(a(b(c = "leaf"))))]), "x");
    let env = b.optional_nested("env").unwrap().unwrap();
    assert_eq!(env.require_str("name").unwrap(), "HOME");
    assert!(env.flag("required").unwrap());
    assert_eq!(env.require_int("retries").unwrap(), 3);
    let leaf = b.optional_nested("deep").unwrap().unwrap().optional_nested("a").unwrap().unwrap().optional_nested("b").unwrap().unwrap();
    assert_eq!(leaf.require_str("c").unwrap(), "leaf");
    assert!(b.require_str("env").is_err(), "a nested attribute is not a string");
}

#[test]
fn a_group_may_not_mix_values_and_keys() {
    let attr: Attribute = parse_quote!(#[x(bad("a", k = 1))]);
    let err = AttrBag::from_attr(&attr, "x").unwrap_err();
    assert!(err.to_string().contains("mixes plain values with `key = value`"), "{err}");
}

#[test]
fn a_repeated_key_becomes_a_list_and_single_readers_say_so() {
    let b = bag(parse_quote!(#[x(alias = "a", alias = "b", alias = "c", once = 1)]), "x");
    assert_eq!(b.get_list_str("alias").unwrap(), ["a", "b", "c"]);
    let err = b.require_str("alias").unwrap_err();
    assert_eq!(err.to_string(), "`alias` given 3 times; expected once");
    assert_eq!(b.require_int("once").unwrap(), 1);
}

#[test]
fn several_attributes_with_the_same_name_merge_like_repeated_keys() {
    let attrs: Vec<Attribute> = vec![parse_quote!(#[x(a = 1, tag = "one")]), parse_quote!(#[x(b = 2, tag = "two")]), parse_quote!(#[other(z)])];
    let b = AttrBag::from_attrs(&attrs, "x").unwrap();
    assert_eq!(b.require_int("a").unwrap(), 1);
    assert_eq!(b.require_int("b").unwrap(), 2);
    assert_eq!(b.get_list_str("tag").unwrap(), ["one", "two"]);
    assert!(!b.contains_key("z"));
}

#[test]
fn keys_may_be_keywords_or_paths() {
    let b = bag(parse_quote!(#[x(type = "t", r#async = 2, serde::rename = "n")]), "x");
    assert_eq!(b.require_str("type").unwrap(), "t");
    assert_eq!(b.require_int("async").unwrap(), 2);
    assert_eq!(b.require_str("serde::rename").unwrap(), "n");
}

#[test]
fn trailing_commas_and_empty_attributes_are_fine() {
    let b = bag(parse_quote!(#[x(a = 1,)]), "x");
    assert_eq!(b.len(), 1);
    let b = bag(parse_quote!(#[x()]), "x");
    assert!(b.is_empty());
    let b = bag(parse_quote!(#[x]), "x");
    assert!(b.is_empty());
}

#[test]
fn the_wrong_attribute_name_and_the_wrong_shape_are_refused() {
    let attr: Attribute = parse_quote!(#[y(a = 1)]);
    assert_eq!(AttrBag::from_attr(&attr, "x").unwrap_err().to_string(), "expected #[x(...)] attribute");
    let attr: Attribute = parse_quote!(#[x = "value"]);
    assert!(AttrBag::from_attr(&attr, "x").unwrap_err().to_string().contains("not #[x = ...]"));
    let attr: Attribute = parse_quote!(#[x(a =)]);
    assert!(AttrBag::from_attr(&attr, "x").is_err());
    let attr: Attribute = parse_quote!(#[x(1 = 2)]);
    assert!(AttrBag::from_attr(&attr, "x").unwrap_err().to_string().contains("expected an attribute key"));
}

#[test]
fn missing_and_mismatched_keys_read_as_errors_not_panics() {
    let b = bag(parse_quote!(#[x(n = 1)]), "x");
    assert_eq!(b.require_str("n").unwrap_err().to_string(), "expected string for `n`, got integer");
    assert_eq!(b.require_str("gone").unwrap_err().to_string(), "missing required attribute key `gone` (string)");
    assert_eq!(b.optional_str("n").unwrap_err().to_string(), "expected string for `n`, got integer");
    assert!(b.span_of("n").is_some());
    assert!(b.span_of("gone").is_none());
}

#[test]
fn insert_replaces_and_entries_iterate_in_key_order() {
    let mut b = bag(parse_quote!(#[x(b = 2, a = 1)]), "x");
    assert_eq!(b.keys().cloned().collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(b.insert("a", AttrValue::from("one")), Some(AttrValue::Int(1)));
    assert_eq!(b.entries().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(b.require_str("a").unwrap(), "one");
}

#[test]
fn a_repeated_key_inside_one_attribute_survives_from_attrs() {
    let attrs: Vec<Attribute> = vec![parse_quote!(#[x(flag = "a", flag = "b")])];
    let b = AttrBag::from_attrs(&attrs, "x").unwrap();
    assert_eq!(b.require_str("flag").unwrap_err().to_string(), "`flag` given 2 times; expected once");
    assert_eq!(b.get_list_str("flag").unwrap(), ["a", "b"]);
}
