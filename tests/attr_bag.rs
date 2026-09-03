use simple_impl_attr_kit::{AttrBag, AttrExpected, AttrSchema, AttrValue};
use syn::{Attribute, parse_quote};

#[test]
fn parses_builder_attr_flags_and_string_values() {
    let attr: Attribute = parse_quote!(#[builder(method = "message", into)]);
    let bag = AttrBag::from_attr(&attr, "builder").unwrap();

    assert_eq!(bag.optional_str("method").unwrap(), Some("message"));
    assert!(bag.flag("into").unwrap());
}

#[test]
fn parses_arg_key_value_attr() {
    let attr: Attribute = parse_quote!(#[arg(kv = "-m")]);
    let bag = AttrBag::from_attr(&attr, "arg").unwrap();

    assert_eq!(bag.require_str("kv").unwrap(), "-m");
}

#[test]
fn parses_token_values_for_type_like_entries() {
    let attr: Attribute = parse_quote!(#[composite(command = "add", method = "add", ty = GitRemoteAdd)]);
    let bag = AttrBag::from_attr(&attr, "composite").unwrap();

    assert_eq!(bag.require_str("command").unwrap(), "add");
    assert_eq!(bag.require_str("method").unwrap(), "add");
    assert_eq!(bag.optional_tokens("ty").unwrap(), Some("GitRemoteAdd"));
}

#[test]
fn schema_rejects_unknown_keys() {
    let attr: Attribute = parse_quote!(#[builder(method = "message", into, wat = true)]);
    let bag = AttrBag::from_attr(&attr, "builder").unwrap();
    let schema = AttrSchema::new()
        .optional("method", AttrExpected::String)
        .optional("into", AttrExpected::Bool);

    assert!(schema.validate(&bag).is_err());
}

#[test]
fn schema_supplies_defaults() {
    let attr: Attribute = parse_quote!(#[builder(into)]);
    let bag = AttrBag::from_attr(&attr, "builder").unwrap();
    let schema = AttrSchema::new()
        .optional("into", AttrExpected::Bool)
        .with_default("method", AttrExpected::String, AttrValue::from("message"));

    let validated = schema.validate(&bag).unwrap();
    assert_eq!(validated.require_str("method").unwrap(), "message");
    assert!(validated.require_bool("into").unwrap());
}

#[test]
fn parses_composite_init_arg_list_as_string_payload() {
    let attr: syn::Attribute = syn::parse_quote!(#[composite(command = "add", ty = GitRemoteAdd, init = "name,url")]);
    let bag = simple_impl_attr_kit::AttrBag::from_attr(&attr, "composite").unwrap();

    assert_eq!(bag.require_str("command").unwrap(), "add");
    assert_eq!(bag.optional_tokens("ty").unwrap(), Some("GitRemoteAdd"));
    assert_eq!(bag.require_str("init").unwrap(), "name,url");
}
