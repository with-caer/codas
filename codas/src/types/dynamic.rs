//! ## Unstable
use alloc::vec::Vec;

use crate::codec::{
    self, CodecError, DataFormat, DataHeader, Decodable, Encodable, Format, ReadsDecodable,
    WritesEncodable,
};

use super::Text;

/// A value whose type is not specified.
///
/// Every coda has an `Unspecified` data type
/// with ordinal `0`. Data of this type is used
/// as the default data for every coda.
///
/// The exact _contents_ of this data are
/// entirely unspecified; they could be "null"
/// or empty (the most common case), or could
/// contain an undocumented sequence of data.
/// That's why we call this type `Unspecified`
/// instead of something like `Null` or `Void`.
#[derive(Default, Debug, Clone, PartialEq)]
pub enum Unspecified {
    /// No value.
    #[default]
    None,

    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    Text(Text),

    /// List of dynamic values.
    List(Vec<Unspecified>),

    /// Mapping of dynamic values (parallel key/value vecs).
    Map {
        keys: Vec<Unspecified>,
        values: Vec<Unspecified>,
    },

    /// Opaque round-tripping of user-defined types.
    /// The `raw` bytes contain the complete payload
    /// (blob + all data field headers and data) verbatim.
    Data {
        format: DataFormat,
        raw: Vec<u8>,
    },
}

/// Ordinal-to-type-tag constants for self-describing encoding.
/// System/built-in ordinals count down from 255 (high end of u8).
/// User-defined ordinals start at 1 (low end).
/// Both ranges grow toward the middle, maximizing the gap.
/// Ordinal 0 = Unspecified/None.
pub(crate) const ORD_NONE: u8 = 0;
pub(crate) const ORD_U8: u8 = 255;
pub(crate) const ORD_U16: u8 = 254;
pub(crate) const ORD_U32: u8 = 253;
pub(crate) const ORD_U64: u8 = 252;
pub(crate) const ORD_I8: u8 = 251;
pub(crate) const ORD_I16: u8 = 250;
pub(crate) const ORD_I32: u8 = 249;
pub(crate) const ORD_I64: u8 = 248;
pub(crate) const ORD_F32: u8 = 247;
pub(crate) const ORD_F64: u8 = 246;
pub(crate) const ORD_BOOL: u8 = 245;
pub(crate) const ORD_TEXT: u8 = 244;
/// Used by the Type enum codec to round-trip a [`super::DataType`] descriptor.
/// Not used by [`Unspecified`], which preserves user-defined ordinals directly.
pub(crate) const ORD_DATA: u8 = 243;
pub(crate) const ORD_LIST: u8 = 242;
pub(crate) const ORD_MAP: u8 = 241;

impl Unspecified {
    /// Constant [`DataType`] for unspecified data.
    pub const DATA_TYPE: super::DataType = super::DataType::new_fluid(
        Text::from("Unspecified"),
        Some(Text::from("Unspecified data.")),
    );

    /// Returns the default value of a `typing`.
    pub fn default_of(typing: &super::Type) -> Unspecified {
        match typing {
            super::Type::Unspecified => Unspecified::None,
            super::Type::U8 => Unspecified::U8(0),
            super::Type::I8 => Unspecified::I8(0),
            super::Type::U16 => Unspecified::U16(0),
            super::Type::I16 => Unspecified::I16(0),
            super::Type::U32 => Unspecified::U32(0),
            super::Type::I32 => Unspecified::I32(0),
            super::Type::U64 => Unspecified::U64(0),
            super::Type::I64 => Unspecified::I64(0),
            super::Type::F32 => Unspecified::F32(0.0),
            super::Type::F64 => Unspecified::F64(0.0),
            super::Type::Bool => Unspecified::Bool(false),
            super::Type::Text => Unspecified::Text(Text::default()),
            super::Type::Data(typing) => Unspecified::Data {
                format: typing.format().as_data_format(),
                raw: Vec::new(),
            },
            super::Type::List(_) => Unspecified::List(Vec::new()),
            super::Type::Map(_) => Unspecified::Map {
                keys: Vec::new(),
                values: Vec::new(),
            },
        }
    }

    /// Returns the type-tag ordinal for this value.
    fn type_ordinal(&self) -> u8 {
        match self {
            Unspecified::None => ORD_NONE,
            Unspecified::U8(_) => ORD_U8,
            Unspecified::I8(_) => ORD_I8,
            Unspecified::U16(_) => ORD_U16,
            Unspecified::I16(_) => ORD_I16,
            Unspecified::U32(_) => ORD_U32,
            Unspecified::I32(_) => ORD_I32,
            Unspecified::U64(_) => ORD_U64,
            Unspecified::I64(_) => ORD_I64,
            Unspecified::F32(_) => ORD_F32,
            Unspecified::F64(_) => ORD_F64,
            Unspecified::Bool(_) => ORD_BOOL,
            Unspecified::Text(_) => ORD_TEXT,
            Unspecified::List(_) => ORD_LIST,
            Unspecified::Map { .. } => ORD_MAP,
            Unspecified::Data { format, .. } => format.ordinal,
        }
    }

    /// Returns the blob size for scalar types.
    fn scalar_blob_size(&self) -> u16 {
        match self {
            Unspecified::U8(_) | Unspecified::I8(_) | Unspecified::Bool(_) => 1,
            Unspecified::U16(_) | Unspecified::I16(_) => 2,
            Unspecified::U32(_) | Unspecified::I32(_) | Unspecified::F32(_) => 4,
            Unspecified::U64(_) | Unspecified::I64(_) | Unspecified::F64(_) => 8,
            _ => 0,
        }
    }
}

// Encoders ///////////////////////////////////////////////
impl Encodable for Unspecified {
    /// The encoding format of unspecified
    /// data is unspecified (i.e., [`Format::Fluid`]).
    const FORMAT: Format = Format::Fluid;

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        match self {
            Unspecified::None => Ok(()),
            Unspecified::U8(v) => v.encode(writer),
            Unspecified::I8(v) => v.encode(writer),
            Unspecified::U16(v) => v.encode(writer),
            Unspecified::I16(v) => v.encode(writer),
            Unspecified::U32(v) => v.encode(writer),
            Unspecified::I32(v) => v.encode(writer),
            Unspecified::U64(v) => v.encode(writer),
            Unspecified::I64(v) => v.encode(writer),
            Unspecified::F32(v) => v.encode(writer),
            Unspecified::F64(v) => v.encode(writer),
            Unspecified::Bool(v) => v.encode(writer),
            Unspecified::Text(v) => v.encode(writer),
            Unspecified::List(items) => {
                for item in items {
                    writer.write_data(item)?;
                }
                Ok(())
            }
            Unspecified::Map { keys, values } => {
                // Encode keys as a self-describing list.
                encode_unspecified_list(keys, writer)?;
                // Encode values as a self-describing list.
                encode_unspecified_list(values, writer)?;
                Ok(())
            }
            Unspecified::Data { raw, .. } => {
                writer.write_all(raw)?;
                Ok(())
            }
        }
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        match self {
            Unspecified::None => Ok(()),

            // Scalars: header with type-tagged ordinal.
            Unspecified::U8(_)
            | Unspecified::I8(_)
            | Unspecified::U16(_)
            | Unspecified::I16(_)
            | Unspecified::U32(_)
            | Unspecified::I32(_)
            | Unspecified::U64(_)
            | Unspecified::I64(_)
            | Unspecified::F32(_)
            | Unspecified::F64(_)
            | Unspecified::Bool(_) => DataHeader {
                count: 1,
                format: DataFormat {
                    blob_size: self.scalar_blob_size(),
                    data_fields: 0,
                    ordinal: self.type_ordinal(),
                },
            }
            .encode(writer),

            // Text: same wire format as Text::encode_header but with ORD_TEXT.
            Unspecified::Text(v) => DataHeader {
                count: codec::try_count(v.len())?,
                format: DataFormat {
                    blob_size: 1,
                    data_fields: 0,
                    ordinal: ORD_TEXT,
                },
            }
            .encode(writer),

            // List: each item self-describes.
            Unspecified::List(items) => DataHeader {
                count: codec::try_count(items.len())?,
                format: DataFormat {
                    blob_size: 0,
                    data_fields: 1,
                    ordinal: ORD_LIST,
                },
            }
            .encode(writer),

            // Map: 2 data fields (keys list + values list).
            Unspecified::Map { .. } => DataHeader {
                count: 1,
                format: DataFormat {
                    blob_size: 0,
                    data_fields: 2,
                    ordinal: ORD_MAP,
                },
            }
            .encode(writer),

            // Typed: preserve the original format.
            Unspecified::Data { format, .. } => DataHeader {
                count: 1,
                format: *format,
            }
            .encode(writer),
        }
    }
}

/// Encodes a `Vec<Unspecified>` as a self-describing list
/// (header + items), used for map keys/values.
fn encode_unspecified_list(
    items: &[Unspecified],
    writer: &mut (impl WritesEncodable + ?Sized),
) -> Result<(), CodecError> {
    // Write list header.
    DataHeader {
        count: codec::try_count(items.len())?,
        format: DataFormat {
            blob_size: 0,
            data_fields: 1,
            ordinal: ORD_LIST,
        },
    }
    .encode(writer)?;

    // Write each item self-describing.
    for item in items {
        writer.write_data(item)?;
    }

    Ok(())
}

/// Reads a complete data sequence (header + payload) from `reader`,
/// appending all bytes verbatim to `buf`.
fn capture_data(
    reader: &mut (impl ReadsDecodable + ?Sized),
    buf: &mut Vec<u8>,
) -> Result<(), CodecError> {
    // Read and capture the header.
    let header: DataHeader = reader.read_data()?;
    header.encode(buf)?;

    // Capture payload for each count.
    for _ in 0..header.count {
        capture_data_with_format(reader, buf, header.format)?;
    }

    Ok(())
}

/// Reads the payload of data with `format` from `reader`,
/// appending all bytes verbatim to `buf`.
fn capture_data_with_format(
    reader: &mut (impl ReadsDecodable + ?Sized),
    buf: &mut Vec<u8>,
    format: DataFormat,
) -> Result<(), CodecError> {
    // Capture blob bytes.
    if format.blob_size > 0 {
        let start = buf.len();
        buf.resize(start + format.blob_size as usize, 0);
        reader.read_exact(&mut buf[start..])?;
    }

    // Capture data fields recursively.
    for _ in 0..format.data_fields {
        capture_data(reader, buf)?;
    }

    Ok(())
}

// Decoders ///////////////////////////////////////////////
impl Decodable for Unspecified {
    fn decode(
        &mut self,
        reader: &mut (impl ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let header = match header {
            Some(h) => h,
            None => {
                // No header means we were called in a blob context.
                // This shouldn't happen for self-describing Unspecified.
                *self = Unspecified::None;
                return Ok(());
            }
        };

        match header.format.ordinal {
            ORD_NONE => {
                // Skip any data that might be present.
                for _ in 0..header.count {
                    reader.skip_blob(header.format.blob_size as usize)?;
                    for _ in 0..header.format.data_fields {
                        reader.skip_data()?;
                    }
                }
                *self = Unspecified::None;
            }

            ORD_U8 => {
                let mut v = 0u8;
                v.decode(reader, None)?;
                *self = Unspecified::U8(v);
            }
            ORD_U16 => {
                let mut v = 0u16;
                v.decode(reader, None)?;
                *self = Unspecified::U16(v);
            }
            ORD_U32 => {
                let mut v = 0u32;
                v.decode(reader, None)?;
                *self = Unspecified::U32(v);
            }
            ORD_U64 => {
                let mut v = 0u64;
                v.decode(reader, None)?;
                *self = Unspecified::U64(v);
            }
            ORD_I8 => {
                let mut v = 0i8;
                v.decode(reader, None)?;
                *self = Unspecified::I8(v);
            }
            ORD_I16 => {
                let mut v = 0i16;
                v.decode(reader, None)?;
                *self = Unspecified::I16(v);
            }
            ORD_I32 => {
                let mut v = 0i32;
                v.decode(reader, None)?;
                *self = Unspecified::I32(v);
            }
            ORD_I64 => {
                let mut v = 0i64;
                v.decode(reader, None)?;
                *self = Unspecified::I64(v);
            }
            ORD_F32 => {
                let mut v = 0.0f32;
                v.decode(reader, None)?;
                *self = Unspecified::F32(v);
            }
            ORD_F64 => {
                let mut v = 0.0f64;
                v.decode(reader, None)?;
                *self = Unspecified::F64(v);
            }
            ORD_BOOL => {
                let mut v = false;
                v.decode(reader, None)?;
                *self = Unspecified::Bool(v);
            }

            ORD_TEXT => {
                let mut v = Text::default();
                // Pass the header through with ordinal translated to 0
                // since Text::decode expects ordinal 0.
                let text_header = DataHeader {
                    count: header.count,
                    format: DataFormat {
                        blob_size: header.format.blob_size,
                        data_fields: header.format.data_fields,
                        ordinal: 0,
                    },
                };
                v.decode(reader, Some(text_header))?;
                *self = Unspecified::Text(v);
            }

            ORD_LIST => {
                let count = header.count as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    if header.format.data_fields > 0 {
                        // Each item is self-describing (has its own header).
                        let item_header: DataHeader = reader.read_data()?;
                        let mut item = Unspecified::None;
                        item.decode(reader, Some(item_header))?;
                        items.push(item);
                    } else {
                        // Items are blobs without individual headers.
                        reader.skip_blob(header.format.blob_size as usize)?;
                        items.push(Unspecified::None);
                    }
                }
                *self = Unspecified::List(items);
            }

            ORD_MAP => {
                // Read two sub-lists (keys, values).
                let mut keys = Unspecified::None;
                let keys_header: DataHeader = reader.read_data()?;
                keys.decode(reader, Some(keys_header))?;

                let mut values = Unspecified::None;
                let values_header: DataHeader = reader.read_data()?;
                values.decode(reader, Some(values_header))?;

                // Extract the Vec from the decoded lists.
                let keys_vec = match keys {
                    Unspecified::List(v) => v,
                    _ => Vec::new(),
                };
                let values_vec = match values {
                    Unspecified::List(v) => v,
                    _ => Vec::new(),
                };

                *self = Unspecified::Map {
                    keys: keys_vec,
                    values: values_vec,
                };
            }

            // User-defined type — opaque capture.
            ordinal => {
                if header.count != 1 {
                    return Err(CodecError::UnsupportedCount {
                        ordinal,
                        count: header.count,
                    });
                }

                let mut raw = Vec::new();

                // Capture blob bytes.
                if header.format.blob_size > 0 {
                    let blob_size = header.format.blob_size as usize;
                    let start = raw.len();
                    raw.resize(start + blob_size, 0);
                    reader.read_exact(&mut raw[start..])?;
                }

                // Capture data fields (header + payload) verbatim.
                for _ in 0..header.format.data_fields {
                    capture_data(reader, &mut raw)?;
                }

                *self = Unspecified::Data {
                    format: DataFormat {
                        blob_size: header.format.blob_size,
                        data_fields: header.format.data_fields,
                        ordinal,
                    },
                    raw,
                };
            }
        }

        Ok(())
    }
}

// Serde ///////////////////////////////////////////////

#[cfg(feature = "serde")]
impl serde::Serialize for Unspecified {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Unspecified::None => serializer.serialize_unit(),
            Unspecified::U8(v) => v.serialize(serializer),
            Unspecified::I8(v) => v.serialize(serializer),
            Unspecified::U16(v) => v.serialize(serializer),
            Unspecified::I16(v) => v.serialize(serializer),
            Unspecified::U32(v) => v.serialize(serializer),
            Unspecified::I32(v) => v.serialize(serializer),
            Unspecified::U64(v) => v.serialize(serializer),
            Unspecified::I64(v) => v.serialize(serializer),
            Unspecified::F32(v) => v.serialize(serializer),
            Unspecified::F64(v) => v.serialize(serializer),
            Unspecified::Bool(v) => v.serialize(serializer),
            Unspecified::Text(v) => v.serialize(serializer),
            Unspecified::List(items) => {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for elem in items {
                    seq.serialize_element(elem)?;
                }
                seq.end()
            }
            Unspecified::Map { keys, values } => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(keys.len()))?;
                for (key, value) in keys.iter().zip(values.iter()) {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Unspecified::Data { .. } => {
                // Typed data doesn't have a meaningful JSON representation;
                // serialize as unit.
                serializer.serialize_unit()
            }
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Unspecified {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UnspecifiedVisitor)
    }
}

/// Visitor that deserializes any self-describing
/// value into the equivalent [`Unspecified`] variant.
#[cfg(feature = "serde")]
struct UnspecifiedVisitor;

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for UnspecifiedVisitor {
    type Value = Unspecified;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("any value")
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(Unspecified::None)
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(Unspecified::None)
    }

    fn visit_some<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        serde::Deserialize::deserialize(deserializer)
    }

    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Unspecified::Bool(v))
    }

    fn visit_u8<E: serde::de::Error>(self, v: u8) -> Result<Self::Value, E> {
        Ok(Unspecified::U8(v))
    }

    fn visit_u16<E: serde::de::Error>(self, v: u16) -> Result<Self::Value, E> {
        Ok(Unspecified::U16(v))
    }

    fn visit_u32<E: serde::de::Error>(self, v: u32) -> Result<Self::Value, E> {
        Ok(Unspecified::U32(v))
    }

    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        // Normalize to I64 when value fits, for JSON integer interop.
        if let Ok(i) = i64::try_from(v) {
            Ok(Unspecified::I64(i))
        } else {
            Ok(Unspecified::U64(v))
        }
    }

    fn visit_i8<E: serde::de::Error>(self, v: i8) -> Result<Self::Value, E> {
        Ok(Unspecified::I8(v))
    }

    fn visit_i16<E: serde::de::Error>(self, v: i16) -> Result<Self::Value, E> {
        Ok(Unspecified::I16(v))
    }

    fn visit_i32<E: serde::de::Error>(self, v: i32) -> Result<Self::Value, E> {
        Ok(Unspecified::I32(v))
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Unspecified::I64(v))
    }

    fn visit_f32<E: serde::de::Error>(self, v: f32) -> Result<Self::Value, E> {
        Ok(Unspecified::F32(v))
    }

    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(Unspecified::F64(v))
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Unspecified::Text(v.into()))
    }

    fn visit_string<E: serde::de::Error>(self, v: alloc::string::String) -> Result<Self::Value, E> {
        Ok(Unspecified::Text(v.into()))
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut items = Vec::new();
        while let Some(elem) = seq.next_element::<Unspecified>()? {
            items.push(elem);
        }
        Ok(Unspecified::List(items))
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut keys = Vec::new();
        let mut values = Vec::new();
        while let Some((key, value)) = map.next_entry::<Unspecified, Unspecified>()? {
            keys.push(key);
            values.push(value);
        }
        Ok(Unspecified::Map { keys, values })
    }
}

#[cfg(test)]
mod tests {
    use crate::codec::ReadsDecodable;

    use super::*;

    #[test]
    pub fn scalar_round_trips() -> Result<(), CodecError> {
        // Test scalar round-trips.
        let cases: Vec<Unspecified> = alloc::vec![
            Unspecified::U8(42),
            Unspecified::I8(-7),
            Unspecified::U16(1000),
            Unspecified::I16(-500),
            Unspecified::U32(100_000),
            Unspecified::I32(-50_000),
            Unspecified::U64(1_000_000),
            Unspecified::I64(-999_999),
            Unspecified::F32(3.14),
            Unspecified::F64(2.718281828),
            Unspecified::Bool(true),
            Unspecified::Bool(false),
            Unspecified::Text("hello world".into()),
            Unspecified::Text("".into()),
        ];

        for original in &cases {
            let mut bytes = alloc::vec![];
            bytes.write_data(original)?;

            let mut decoded = Unspecified::None;
            let header: DataHeader = (&mut bytes.as_slice()).read_data()?;
            decoded.decode(&mut bytes.as_slice().split_at(8).1, Some(header))?;

            // Simpler: use read_data_into
            let mut decoded2 = Unspecified::None;
            (&mut bytes.as_slice()).read_data_into(&mut decoded2)?;

            assert_eq!(*original, decoded2, "round-trip failed for {original:?}");
        }

        Ok(())
    }

    #[test]
    pub fn list_round_trips() -> Result<(), CodecError> {
        let original = Unspecified::List(alloc::vec![
            Unspecified::I32(1),
            Unspecified::Text("two".into()),
            Unspecified::Bool(true),
        ]);

        let mut bytes = alloc::vec![];
        bytes.write_data(&original)?;

        let mut decoded = Unspecified::None;
        (&mut bytes.as_slice()).read_data_into(&mut decoded)?;

        assert_eq!(original, decoded);

        Ok(())
    }

    #[test]
    pub fn map_round_trips() -> Result<(), CodecError> {
        let original = Unspecified::Map {
            keys: alloc::vec![Unspecified::Text("a".into()), Unspecified::Text("b".into()),],
            values: alloc::vec![Unspecified::I32(1), Unspecified::Bool(true)],
        };

        let mut bytes = alloc::vec![];
        bytes.write_data(&original)?;

        let mut decoded = Unspecified::None;
        (&mut bytes.as_slice()).read_data_into(&mut decoded)?;

        assert_eq!(original, decoded);

        Ok(())
    }

    #[test]
    pub fn typed_round_trips() -> Result<(), CodecError> {
        use super::super::tests::{NestedTestData, TestData};

        // Encode typed data.
        let test_data = TestData {
            number: 1,
            floaty: 60.90,
            text_list: alloc::vec!["one".into(), "two".into()],
            text: "hello".into(),
            nested: NestedTestData { boolean: true },
            two_d: alloc::vec![
                alloc::vec!["three".into(), "four".into()],
                alloc::vec!["five".into(), "six".into()],
            ],
        };
        let mut static_bytes = alloc::vec![];
        static_bytes.write_data(&test_data)?;

        // Decode as Unspecified (should capture as Data).
        let mut decoded = Unspecified::None;
        (&mut static_bytes.as_slice()).read_data_into(&mut decoded)?;
        assert!(matches!(decoded, Unspecified::Data { .. }));

        // Re-encode the Unspecified::Data and verify bytes match.
        let mut re_encoded = alloc::vec![];
        re_encoded.write_data(&decoded)?;
        assert_eq!(
            static_bytes, re_encoded,
            "typed round-trip bytes must match"
        );

        // Verify the re-encoded bytes decode back to the original typed data.
        let roundtripped: TestData = re_encoded.as_slice().read_data()?;
        assert_eq!(test_data, roundtripped);

        Ok(())
    }

    #[test]
    pub fn none_is_zero_bytes() -> Result<(), CodecError> {
        let none = Unspecified::None;
        let mut bytes = alloc::vec![];
        bytes.write_data(&none)?;
        assert_eq!(0, bytes.len(), "None should encode to 0 bytes");
        Ok(())
    }
}
