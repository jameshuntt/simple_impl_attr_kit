use simple_impl_attr_kit::{
    parse_field_validation_attrs, parse_validation_attrs, ParsedValidationRule,
};
use syn::{parse_quote, Data, DeriveInput, Field, Fields};

fn first_named_field(input: DeriveInput) -> Field {
    let Data::Struct(data) = input.data else {
        panic!("expected struct");
    };
    let Fields::Named(fields) = data.fields else {
        panic!("expected named fields");
    };
    fields.named.into_iter().next().expect("missing field")
}

#[test]
fn parses_field_dependency_validation_attrs() {
    let input: DeriveInput = parse_quote! {
        pub struct Example {
            #[requires("profile")]
            #[invalid_without("force")]
            #[conflicts_with("yaml")]
            delete: bool,
        }
    };
    let field = first_named_field(input);

    let spec = parse_field_validation_attrs("delete", &field.attrs).unwrap();

    assert_eq!(
        spec.rules(),
        &[
            ParsedValidationRule::Requires {
                field: "delete".into(),
                required: "profile".into(),
            },
            ParsedValidationRule::InvalidWithout {
                field: "delete".into(),
                required: "force".into(),
            },
            ParsedValidationRule::ConflictsWith {
                field: "delete".into(),
                conflicts_with: "yaml".into(),
            },
        ]
    );
}

#[test]
fn parses_only_pair_with_on_pairing_field() {
    let input: DeriveInput = parse_quote! {
        pub struct Example {
            #[only_pair_with("delete")]
            force: bool,
        }
    };
    let field = first_named_field(input);

    let spec = parse_field_validation_attrs("force", &field.attrs).unwrap();

    assert_eq!(
        spec.rules(),
        &[ParsedValidationRule::OnlyPairWith {
            field: "force".into(),
            paired_with: "delete".into(),
        }]
    );
}

#[test]
fn parses_custom_validation_hook_attrs() {
    let input: DeriveInput = parse_quote! {
        pub struct Example {
            #[validate(with = "validate_cp_preflight")]
            _validate: (),
        }
    };
    let field = first_named_field(input);

    let spec = parse_field_validation_attrs("_validate", &field.attrs).unwrap();

    assert_eq!(
        spec.rules(),
        &[ParsedValidationRule::CustomFunction {
            function_path: "validate_cp_preflight".into(),
        }]
    );
}

#[test]
fn parses_command_set_validation_attrs() {
    let input: DeriveInput = parse_quote! {
        #[one_of("json", "yaml")]
        #[at_least_one_of("source", "destination")]
        pub struct Example;
    };

    let spec = parse_validation_attrs(&input.attrs).unwrap();

    assert_eq!(
        spec.rules(),
        &[
            ParsedValidationRule::OneOf {
                fields: vec!["json".into(), "yaml".into()],
            },
            ParsedValidationRule::AtLeastOneOf {
                fields: vec!["source".into(), "destination".into()],
            },
        ]
    );
}

#[test]
fn validation_hook_rejects_unknown_validate_keys() {
    let input: DeriveInput = parse_quote! {
        pub struct Example {
            #[validate(func = "validate_cp_preflight")]
            _validate: (),
        }
    };
    let field = first_named_field(input);

    let err = parse_field_validation_attrs("_validate", &field.attrs).unwrap_err();
    assert!(err.to_string().contains("unknown attribute key `func`"));
}

#[test]
fn dependency_attrs_require_string_literal_targets() {
    let input: DeriveInput = parse_quote! {
        pub struct Example {
            #[invalid_without(force)]
            delete: bool,
        }
    };
    let field = first_named_field(input);

    assert!(parse_field_validation_attrs("delete", &field.attrs).is_err());
}
