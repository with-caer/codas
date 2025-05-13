use ::prost::Message;
use codas::codec::{ReadsDecodable, WritesEncodable};
use codas_macros::export_coda;
use criterion::{criterion_group, criterion_main, Criterion};

mod prost_boat;

export_coda!("codas/benches/sailboat.md");

fn codecs(c: &mut Criterion) {
    let mut group = c.benchmark_group("Codecs");
    group.throughput(criterion::Throughput::Elements(1));

    // Create a Codas sailboat.
    let codas_boat = Boat {
        name: "In Amber Clad".into(),
        seaworthy: true,
        sail: Sail {
            surface_area: 13.0,
            sail_count: 2,
            breaking_strength: 33.7,
            surface_material: "Carbon Fiber".into(),
            rigging_material: "Dyneema".into(),
        },
        hull: Hull {
            serial_number: 142,
            length: 4.7,
            manufacturer_id: "UNSC".into(),
            manufacture_year: "2547".into(),
            model_year: "2515".into(),
        },
    };

    // Pre-encode it's bytes for decoding later.
    let mut codas_boat_bytes = vec![];
    codas_boat_bytes.write_data(&codas_boat).unwrap();
    let codas_boat_bytes = codas_boat_bytes;
    assert_eq!(codas_boat, codas_boat_bytes.as_slice().read_data().unwrap());

    // Encoding (Codas)
    group.bench_function("Codas - Encode", |b| {
        let mut bytes = vec![];

        b.iter(|| {
            bytes.clear();
            bytes.write_data(&codas_boat).unwrap();
            assert_eq!(codas_boat_bytes, bytes);
        });
    });

    // Decoding (Codas)
    group.bench_function("Codas - Decode", |b| {
        let mut boat = Boat::default();
        b.iter(|| {
            codas_boat_bytes
                .as_slice()
                .read_data_into(&mut boat)
                .unwrap();
            assert_eq!(codas_boat, boat);
        });
    });

    // Create a Prost sailboat.
    let prost_boat = prost_boat::Boat {
        name: "In Amber Clad".into(),
        seaworthy: true,
        sail: Some(prost_boat::Sail {
            surface_area: 13.0,
            sail_count: 2,
            breaking_strength: 33.7,
            surface_material: "Carbon Fiber".into(),
            rigging_material: "Dyneema".into(),
        }),
        hull: Some(prost_boat::Hull {
            serial_number: 142,
            length: 4.7,
            manufacturer_id: "UNSC".into(),
            manufacture_year: "2547".into(),
            model_year: "2515".into(),
        }),
    };

    // Pre-encode it's bytes for decoding later.
    let mut prost_boat_bytes = vec![];
    prost_boat.encode(&mut prost_boat_bytes).unwrap();
    let prost_boat_bytes = prost_boat_bytes;
    assert_eq!(
        prost_boat,
        prost_boat::Boat::decode(prost_boat_bytes.as_slice()).unwrap()
    );

    // Encoding (Prost)
    group.bench_function("Proto3 (Prost) - Encode", |b| {
        let mut bytes = vec![];

        b.iter(|| {
            bytes.clear();
            prost_boat.encode(&mut bytes).unwrap();
            assert_eq!(prost_boat_bytes, bytes);
        });
    });

    // Decoding (Prost)
    group.bench_function("Proto3 (Prost) - Decode", |b| {
        let mut boat = prost_boat::Boat::default();
        b.iter(|| {
            boat.merge(prost_boat_bytes.as_slice()).unwrap();
            assert_eq!(prost_boat, boat);
        });
    });
}

// Create a new group named `benches` and
// run it with all benchmark methods.
criterion_group!(benches, codecs);
criterion_main!(benches);
