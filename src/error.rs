//! One helper for errors that point at tokens.

use quote::ToTokens;

/// An error that points at `tokens`.
pub fn spanned_error(tokens: impl ToTokens, message: impl Into<String>) -> syn::Error {
    syn::Error::new_spanned(tokens, message.into())
}
