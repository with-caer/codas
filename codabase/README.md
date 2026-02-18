[![`codabase` on crates.io](https://img.shields.io/crates/v/codabase)](https://crates.io/crates/codas)
[![`codabase` on docs.rs](https://img.shields.io/docsrs/codabase)](https://docs.rs/codas/)
[![`codabase` is FSL-1.1-MIT licensed](https://img.shields.io/badge/license-FSL--1.1--MIT-yellow.svg)](LICENSE.md)

The Codabase development platform.

## What's Here

`codabase` is a CLI for working with [Codas](https://crates.io/crates/codas): Compiling coda markdown into language-specific bindings, inspecting binary coda-encoded data, and running cryptography utilities.

## Writing a Coda

Codas are defined in Markdown. A coda begins with a `#` header containing its name followed by `Coda`. Data types are `##` headers followed by `Data`. Fields are `+` list items with a name and type.

```markdown
# `Greeter` Coda
An example coda.

## `Request` Data
A greeting request.

+ `message` text

  The greeting message.

## `Response` Data
A greeting response.

+ `message` text
+ `original_request` Request
```

Indented text after a `+` field becomes the field's
documentation. Type references in fields are bare names
(`Request`, not `` `Request` ``).

### Field Types

Type | Syntax
-----|-------
Unsigned integers | `u8`, `u16`, `u32`, `u64`
Signed integers | `i8`, `i16`, `i32`, `i64`
Floating-point | `f32`, `f64`
Boolean | `bool`
Text | `text`
Nested data | `DataTypeName`
List | `list of <type>`
Map | `map of <key_type> to <value_type>`
Optional | `optional <type>`
Unspecified (dynamic) | `unspecified`

### Rules

- The _order_ of data types and fields matters. Reordering
  changes the binary encoding.
- New data types can be added to the end of a coda.
- New fields can be added to the end of a data type.
- Existing fields and data types can be renamed freely.

## Compiling Codas

### Single Coda to stdout

Compile one coda to a specific language. `--source`
accepts a file or directory.

```sh
codabase compile --source greeter.md --lang rust
codabase compile --source greeter.md --lang python
```

A file that doesn't start with `` # `Name` Coda `` will
error in single-file mode (and be skipped in batch mode).

Accepts input from stdin:

```sh
cat greeter.md | codabase compile --lang typescript
```

Supported languages: `rust`, `python`, `typescript`,
`open-api`, `sql`.

### Batch Compilation

Compile all codas in a directory to all languages:

```sh
codabase compile --source ./codas --target ./generated
```

This recursively discovers `.md` files in `--source`,
parses each as a coda (skipping non-coda markdown),
and writes output to `--target/<lang>/`:

```
generated/
  rust/greeter.rs
  python/greeter.py
  typescript/greeter.ts
  open-api/greeter.yaml
  sql/greeter.sql
```

When `--source` is omitted, the current directory is
used. When `--target` is omitted, `./target` is used.

## License

Copyright © 2024 - 2026 With Caer, LLC.

Licensed under the Functional Source License, Version 1.1, MIT Future License.
Refer to [the license file](LICENSE.md) for more info.
