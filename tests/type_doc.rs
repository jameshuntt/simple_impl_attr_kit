use simple_impl_attr_kit::{
    extract_doc_comments,
    generic_inner_ty,
    is_bool_ty,
    is_option_ty,
    is_os_string_ty,
    is_vec_ty,
    type_last_ident,
};
use syn::{DeriveInput, Type, parse_quote};

#[test]
fn recognizes_common_field_types() {
    let option: Type = parse_quote!(Option<std::ffi::OsString>);
    let vec: Type = parse_quote!(Vec<String>);
    let bool_ty: Type = parse_quote!(bool);
    let os: Type = parse_quote!(std::ffi::OsString);

    assert!(is_option_ty(&option));
    assert!(is_vec_ty(&vec));
    assert!(is_bool_ty(&bool_ty));
    assert!(is_os_string_ty(&os));

    let inner = generic_inner_ty(&option).unwrap();
    assert_eq!(type_last_ident(inner).unwrap().to_string(), "OsString");
}

#[test]
fn extracts_doc_comments() {
    let input: DeriveInput = parse_quote! {
        /// first line
        /// second line
        struct Example;
    };

    assert_eq!(
        extract_doc_comments(&input.attrs).as_deref(),
        Some("first line\nsecond line")
    );
}
