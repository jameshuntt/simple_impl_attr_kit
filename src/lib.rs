//! The key-value grammar behind a derive macro's attributes.
//!
//! Every derive that takes `#[name(key = value, ...)]` has to parse the same
//! things: bare keys, literals of every kind, types and paths, lists, nested
//! keys, keys given twice, and then check that what was written is what the
//! macro accepts, and say where it went wrong. This crate is that parser and
//! that checker, with no opinion about what the keys mean.
//!
//! * [`AttrBag`] parses one attribute (or several with the same name) into
//!   keys and [`AttrValue`]s, remembering each key's span.
//! * [`AttrSchema`] says which keys exist, what kind they hold, which are
//!   required, repeatable or limited to a set of choices, which imply or
//!   exclude each other, and what an old name maps to. Its errors point at
//!   the key and name the nearest known key.
//! * [`type_read`] answers the small questions about field types a derive
//!   asks (`Option<_>`? `Vec<_>`? `bool`?), [`doc`] gathers `///` comments,
//!   and [`validation`] parses the `requires` / `conflicts_with` /
//!   `one_of` family of field attributes.
//!
//! ```
//! use simple_impl_attr_kit::{AttrBag, AttrExpected, AttrSchema, AttrValue};
//! use syn::parse_quote;
//!
//! let attr: syn::Attribute = parse_quote!(
//!     #[arg(long = "verbose", short = 'v', count, default = 0, aliases("v", "verb"), env(name = "VERBOSE", required))]
//! );
//! let bag = AttrBag::from_attr(&attr, "arg").unwrap();
//!
//! assert_eq!(bag.require_str("long").unwrap(), "verbose");
//! assert_eq!(bag.optional_char("short").unwrap(), Some('v'));
//! assert!(bag.flag("count").unwrap());
//! assert_eq!(bag.require_int("default").unwrap(), 0);
//! assert_eq!(bag.get_list_str("aliases").unwrap(), ["v", "verb"]);
//! let env = bag.optional_nested("env").unwrap().unwrap();
//! assert_eq!(env.require_str("name").unwrap(), "VERBOSE");
//! assert!(env.flag("required").unwrap());
//!
//! let schema = AttrSchema::new()
//!     .required("long", AttrExpected::String)
//!     .optional("short", AttrExpected::Char)
//!     .optional("count", AttrExpected::Bool)
//!     .with_default("default", AttrExpected::Int, AttrValue::from(0i64))
//!     .repeatable("aliases", AttrExpected::String)
//!     .optional("env", AttrExpected::Nested);
//! assert!(schema.validate(&bag).is_ok());
//! ```
//!
//! The grammar, in full:
//!
//! | written | value |
//! |---|---|
//! | `key` | `Bool(true)` |
//! | `key = true`, `key = false` | `Bool` |
//! | `key = "text"` | `Str` |
//! | `key = 3`, `key = -3` | `Int` |
//! | `key = 1.5`, `key = -0.5` | `Float` |
//! | `key = 'c'` | `Char` |
//! | `key = b"raw"` | `Bytes` |
//! | `key = Vec<u8>`, `key = a::b::C`, `key = f(1, 2)` | `Tokens`, verbatim |
//! | `key(a, "b", 3)`, `key = [a, "b", 3]`, `key()` | `List` |
//! | `key(inner = 1, flag)` | `Nested` |
//! | `key = 1, key = 2` | `List` of both, marked as repeated |

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod bag;
pub mod doc;
pub mod error;
pub mod schema;
pub mod type_read;
pub mod validation;
pub mod value;

pub use bag::{AttrBag, Entry};
pub use doc::extract_doc_comments;
pub use error::spanned_error;
pub use schema::{AttrExpected, AttrSchema};
pub use type_read::{generic_inner_ty, is_bool_ty, is_option_ty, is_os_string_ty, is_string_ty, is_vec_ty, type_last_ident};
pub use validation::{parse_field_validation_attrs, parse_validation_attrs, ParsedValidationRule, ParsedValidationSpec};
pub use value::AttrValue;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_doctests {}
