[![`codas-macros` on crates.io](https://img.shields.io/crates/v/codas-macros)](https://crates.io/crates/codas-macros)
[![`codas-macros` on docs.rs](https://img.shields.io/docsrs/codas-macros)](https://docs.rs/codas-macros/)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE.txt)

Macros for generating Rust types from [Codas](https://crates.io/crates/codas).

## [`export_coda!`](macro@export_coda)

This macro parses a coda from a file path relative
to the crate's _workspace_ root path, and generating
Rust data structures for the coda in-place.

> _Note:_ A crate's workspace root is always the
> top-most directory containing a `Cargo.toml`.

Here's an example that exports Rust data structures
for the [`greeter_coda.md`](tests/greeter_coda.md):

```rust
# use codas::codec::*;
# use codas_macros::export_coda;

// The file path should be relative to
// the _root_ of a crate's workspace.
export_coda!("codas-macros/tests/greeter_coda.md");

# fn main() {
// A struct is generated for each data type in the coda.
let request = Request { message: "Hi!".into() };

// An enum is generated with variants for each data type
// in the coda. The enum's name will be the same as the
// coda's name, with `Data` appended.
let data = GreeterData::from(request.clone());
assert_eq!("Hi!", match data.clone() {
    GreeterData::Request(Request { message }) => message,
    GreeterData::Response(..) | 
    _ => unimplemented!(),
});

// The structs and enums have auto-generated coda codecs.
let mut request_bytes = vec![];
request_bytes.write_data(&request).unwrap();
let mut data_bytes = vec![];
data_bytes.write_data(&data).unwrap();
assert_eq!(request_bytes, data_bytes);

// The enum can safely decode bytes containing
// coda-encoded data.
let data = data_bytes.as_slice().read_data().unwrap();
assert_eq!("Hi!", match data {
    GreeterData::Request(Request { message }) => message,
    GreeterData::Response(..) | 
    _ => unimplemented!(),
});
# }
```

## License

Copyright © 2024—2025 With Caer, LLC and Alicorn Systems, LLC.

Licensed under the MIT license. Refer to [the license file](../LICENSE.txt) for more info.