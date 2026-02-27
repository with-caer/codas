#![cfg(test)]

extern crate alloc;

use alloc::collections::BTreeMap;

use codas::types::{Text, Unspecified};
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
        request_data: BTreeMap::default(),
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

/// Tests that `map of text to unspecified` fields
/// can be manipulated directly via Rust structs.
#[test]
pub fn unspecified_map_via_rust() {
    let mut example = Example::default();

    // Insert scalar values of different types.
    example
        .request_data
        .insert("count".into(), Unspecified::U64(42));
    example
        .request_data
        .insert("label".into(), Unspecified::Text("hello".into()));
    example
        .request_data
        .insert("enabled".into(), Unspecified::Bool(true));
    example
        .request_data
        .insert("ratio".into(), Unspecified::F64(3.14));

    assert_eq!(example.request_data.len(), 4);
    assert_eq!(
        example.request_data.get(&Text::from("count")),
        Some(&Unspecified::U64(42))
    );
    assert_eq!(
        example.request_data.get(&Text::from("label")),
        Some(&Unspecified::Text("hello".into()))
    );
    assert_eq!(
        example.request_data.get(&Text::from("enabled")),
        Some(&Unspecified::Bool(true))
    );

    // None represents absent data.
    example
        .request_data
        .insert("empty".into(), Unspecified::None);
    assert_eq!(
        example.request_data.get(&Text::from("empty")),
        Some(&Unspecified::None)
    );

    // Wrap in the coda enum.
    let data = FullData::Example(example);
    assert!(matches!(data, FullData::Example(..)));
}

/// Tests that `map of text to unspecified` fields
/// round-trip correctly through JSON via serde.
#[test]
pub fn unspecified_map_via_json() {
    let mut example = Example::default();
    example
        .request_data
        .insert("user".into(), Unspecified::Text("alice".into()));
    example
        .request_data
        .insert("age".into(), Unspecified::I64(30));
    example
        .request_data
        .insert("active".into(), Unspecified::Bool(true));

    let data = FullData::Example(example);

    // Serialize to JSON.
    let json_value = serde_json::to_value(&data).unwrap();

    // The request_data map should appear as a JSON object
    // nested under the "Example" variant.
    let request_data = &json_value["Example"]["request_data"];
    assert_eq!(request_data["user"], json!("alice"));
    assert_eq!(request_data["age"], json!(30));
    assert_eq!(request_data["active"], json!(true));

    // Deserialize back from JSON.
    let roundtripped: FullData = serde_json::from_value(json_value).unwrap();
    assert_eq!(data, roundtripped);
}

/// Tests that arbitrary JSON can be deserialized into
/// an `Unspecified` map, including nested structures.
#[test]
pub fn unspecified_map_from_arbitrary_json() {
    let input = json!({
        "Example": {
            "num_1": 0, "num_2": 0, "num_3": 0, "num_4": 0,
            "num_5": 0, "num_6": 0, "num_7": 0, "num_8": 0,
            "num_9": 0.0, "num_10": 0.0,
            "boolean": false,
            "message": "",
            "listing": [],
            "mapping": {},
            "optional_message": null,
            "request_data": {
                "name": "test",
                "tags": ["alpha", "beta"],
                "metadata": {
                    "version": 2,
                    "draft": false
                }
            }
        }
    });

    let data: FullData = serde_json::from_value(input).unwrap();
    let example = match &data {
        FullData::Example(e) => e,
        _ => panic!("expected Example variant"),
    };

    // Scalar value.
    assert_eq!(
        example.request_data.get(&Text::from("name")),
        Some(&Unspecified::Text("test".into()))
    );

    // Nested array — deserialized as Unspecified::List.
    let tags = example.request_data.get(&Text::from("tags"));
    assert!(
        matches!(tags, Some(Unspecified::List(..))),
        "expected List, got {tags:?}"
    );

    // Nested object — deserialized as Unspecified::Map.
    let metadata = example.request_data.get(&Text::from("metadata"));
    assert!(
        matches!(metadata, Some(Unspecified::Map { .. })),
        "expected Map, got {metadata:?}"
    );

    // Round-trip the nested structure back to JSON
    // and verify it survives the trip.
    let json_out = serde_json::to_value(&data).unwrap();
    let rd = &json_out["Example"]["request_data"];
    assert_eq!(rd["name"], json!("test"));
    assert_eq!(rd["tags"], json!(["alpha", "beta"]));
    assert_eq!(rd["metadata"]["version"], json!(2));
    assert_eq!(rd["metadata"]["draft"], json!(false));
}
