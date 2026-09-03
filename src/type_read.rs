//! The small questions a derive asks about a field type.

use syn::{GenericArgument, Ident, PathArguments, Type};

/// The last path segment of a type: `Vec` for `std::vec::Vec<u8>`.
pub fn type_last_ident(ty: &Type) -> Option<&Ident> {
    match ty {
        Type::Path(type_path) => type_path.path.segments.last().map(|segment| &segment.ident),
        _ => None,
    }
}

/// Whether the type is `Option<_>` (by its last segment).
pub fn is_option_ty(ty: &Type) -> bool {
    type_last_ident(ty).is_some_and(|ident| ident == "Option") && generic_inner_ty(ty).is_some()
}

/// Whether the type is `Vec<_>` (by its last segment).
pub fn is_vec_ty(ty: &Type) -> bool {
    type_last_ident(ty).is_some_and(|ident| ident == "Vec") && generic_inner_ty(ty).is_some()
}

/// Whether the type is `bool`.
pub fn is_bool_ty(ty: &Type) -> bool {
    type_last_ident(ty).is_some_and(|ident| ident == "bool")
}

/// Whether the type is `String` (by its last segment).
pub fn is_string_ty(ty: &Type) -> bool {
    type_last_ident(ty).is_some_and(|ident| ident == "String")
}

/// Whether the type is `OsString` (by its last segment).
pub fn is_os_string_ty(ty: &Type) -> bool {
    type_last_ident(ty).is_some_and(|ident| ident == "OsString")
}

/// The first type argument of a generic type: `u8` for `Vec<u8>` or `Option<u8>`.
pub fn generic_inner_ty(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };

    args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}
