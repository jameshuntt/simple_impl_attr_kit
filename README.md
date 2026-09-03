# simple_impl_attr_kit

The key-value grammar behind a derive macro's attributes.

Every derive that takes `#[name(key = value, ...)]` has to parse the same
things: bare keys, literals of every kind, types and paths, lists, nested
keys, keys given twice, and then check that what was written is what the
macro accepts, and say where it went wrong. This crate is that parser and
that checker, with no opinion about what the keys mean.

```rust
use simple_impl_attr_kit::{AttrBag, AttrExpected, AttrSchema, AttrValue};
use syn::parse_quote;

let attr: syn::Attribute = parse_quote!(
    #[arg(long = "verbose", short = 'v', count, default = 0, aliases("v", "verb"), env(name = "VERBOSE", required))]
);
let bag = AttrBag::from_attr(&attr, "arg")?;

assert_eq!(bag.require_str("long")?, "verbose");
assert_eq!(bag.optional_char("short")?, Some('v'));
assert!(bag.flag("count")?);
assert_eq!(bag.get_list_str("aliases")?, ["v", "verb"]);
assert_eq!(bag.optional_nested("env")?.unwrap().require_str("name")?, "VERBOSE");

let schema = AttrSchema::new()
    .required("long", AttrExpected::String)
    .optional("short", AttrExpected::Char)
    .optional("count", AttrExpected::Bool)
    .with_default("default", AttrExpected::Int, AttrValue::from(0i64))
    .repeatable("aliases", AttrExpected::String)
    .optional("env", AttrExpected::Nested)
    .one_of("style", &["dash", "equals"])
    .conflicts("count", "style");
let checked = schema.validate(&bag)?;
assert_eq!(checked.get_list_str("aliases")?, ["v", "verb"]);

// a typo is refused, pointing at the key, naming the nearest known one
let attr: syn::Attribute = parse_quote!(#[arg(long = "v", cuont)]);
let err = schema.validate(&AttrBag::from_attr(&attr, "arg")?).unwrap_err();
assert!(err.to_string().starts_with("unknown attribute key `cuont`; did you mean `count`?"));
# Ok::<(), syn::Error>(())
```

## The grammar

| written | value |
|---|---|
| `key` | `Bool(true)` |
| `key = true`, `key = false` | `Bool` |
| `key = "text"` | `Str` |
| `key = 3`, `key = -3` | `Int` |
| `key = 1.5`, `key = -0.5` | `Float` |
| `key = 'c'` | `Char` |
| `key = b"raw"` | `Bytes` |
| `key = Vec<u8>`, `key = a::b::C`, `key = f(1, 2)` | `Tokens`, verbatim |
| `key(a, "b", 3)`, `key = [a, "b", 3]`, `key()` | `List` |
| `key(inner = 1, flag)` | `Nested`, a bag of its own |
| `key = 1, key = 2` | `List` of both; a reader that wants one value says "given 2 times" |

Keys may be identifiers, keywords (`type`, `r#async`) or paths
(`serde::rename`). Several attributes with the same name merge like
repeated keys. Every key keeps its span, so an error from a reader or a
schema points at the key, not at the derive.

## The schema

`required`, `optional`, `with_default`, `repeatable` (always a list, empty
when absent), `one_of` (a string from a fixed set), `alias` (an old name
that reads as a new one), `requires` and `conflicts` between keys,
`allow_unknown` to pass extra keys through, `doc` and `describe()` to
print the whole vocabulary for documentation or a help message.

## Also here

`type_read` answers the small questions a derive asks about a field's type
(`Option<_>`? `Vec<_>`? `bool`?), `doc` gathers `///` comments, and
`validation` parses the `requires` / `conflicts_with` / `one_of` family of
field attributes used by `simple_impl_derive`.

## License

MIT OR Apache-2.0.
