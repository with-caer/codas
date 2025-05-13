#![cfg(test)]

extern crate alloc;

use alloc::collections::BTreeMap;

use codas::types::Text;
use codas_macros::export_coda;
use serde_json::json;

export_coda!("codas-macros/tests/full_coda.md");
export_coda!("codas-macros/tests/greeter_coda.md");

/// If this test _compiles_, all data
/// types were generated correctly by
/// [`export_coda`].
#[test]
pub fn generated_types() {
    let _ = FullData::Example(Example {
        num_1: 3u8,
        num_2: 3u16,
        num_3: 3u32,
        num_4: 3u64,
        num_5: -3i8,
        num_6: -3i16,
        num_7: -3i32,
        num_8: -3i64,
        num_9: 3.33f32,
        num_10: 3.33f64,
        boolean: true,
        message: Text::from("Hello!"),
        listing: vec![],
        mapping: BTreeMap::default(),
        optional_message: Some(Text::from("World!")),
    });

    let _ = GreeterData::Response(Response {
        message: Text::from("Hello, world!"),
        original_request: Request {
            message: Text::from("Hi, World!"),
        },
    });

    let nesting = GreeterData::Nesting(Nesting {
        request_id: 1337,
        nested: Request {
            message: "I'm Nested, but flattened to the top!".into(),
        },
    });

    let nesting_json = serde_json::to_value(&nesting).unwrap();

    let expected_json = json!({
        "Nesting": {
            "request_id": 1337,
            "message": "I'm Nested, but flattened to the top!"
          }
    });

    assert_eq!(expected_json, nesting_json);
}
