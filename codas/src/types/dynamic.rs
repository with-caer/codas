//! ## Unstable
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::codec::{
    self, CodecError, DataFormat, DataHeader, Decodable, Encodable, Format, ReadsDecodable,
    WritesEncodable,
};

use super::{Text, Type};

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
    /// The default value of unspecified data.
    #[default]
    Default,

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

    /// Mapping of text keys to dynamic values.
    Map(BTreeMap<Text, Unspecified>),

    /// Opaque round-tripping of user-defined types.
    /// The header preserves the original count and format;
    /// `raw` contains the complete payload (blob + all data
    /// field headers and data) for all `count` entries verbatim.
    Data {
        header: DataHeader,
        raw: Vec<u8>,
    },
}

impl Unspecified {
    /// Constant [`DataType`] for unspecified data.
    pub const DATA_TYPE: super::DataType = super::DataType::new_fluid(
        Text::from("Unspecified"),
        Some(Text::from("Unspecified data.")),
    );

    /// Returns the corresponding [`Type`](Type) for this value.
    pub fn as_type(&self) -> Type {
        match self {
            Unspecified::Default => Type::Unspecified,
            Unspecified::U8(_) => Type::U8,
            Unspecified::I8(_) => Type::I8,
            Unspecified::U16(_) => Type::U16,
            Unspecified::I16(_) => Type::I16,
            Unspecified::U32(_) => Type::U32,
            Unspecified::I32(_) => Type::I32,
            Unspecified::U64(_) => Type::U64,
            Unspecified::I64(_) => Type::I64,
            Unspecified::F32(_) => Type::F32,
            Unspecified::F64(_) => Type::F64,
            Unspecified::Bool(_) => Type::Bool,
            Unspecified::Text(_) => Type::Text,
            Unspecified::List(_) => Type::List(alloc::boxed::Box::new(Type::Unspecified)),
            Unspecified::Map(_) => {
                Type::Map(alloc::boxed::Box::new((Type::Text, Type::Unspecified)))
            }
            // Data and Default don't have a precise Type mapping.
            Unspecified::Data { .. } => Type::Unspecified,
        }
    }

    /// Returns the homogeneous element type if all items
    /// share the same type, or `None` for empty/heterogeneous lists.
    fn homogeneous_element_type(items: &[Unspecified]) -> Option<Type> {
        let first = items.first()?;
        let first_type = first.as_type();
        if items[1..].iter().all(|item| item.as_type() == first_type) {
            Some(first_type)
        } else {
            None
        }
    }

    /// Returns the default value of a `typing`.
    pub fn default_of(typing: &Type) -> Unspecified {
        match typing {
            Type::Unspecified => Unspecified::Default,
            Type::U8 => Unspecified::U8(0),
            Type::I8 => Unspecified::I8(0),
            Type::U16 => Unspecified::U16(0),
            Type::I16 => Unspecified::I16(0),
            Type::U32 => Unspecified::U32(0),
            Type::I32 => Unspecified::I32(0),
            Type::U64 => Unspecified::U64(0),
            Type::I64 => Unspecified::I64(0),
            Type::F32 => Unspecified::F32(0.0),
            Type::F64 => Unspecified::F64(0.0),
            Type::Bool => Unspecified::Bool(false),
            Type::Text => Unspecified::Text(Text::default()),
            Type::Data(typing) => Unspecified::Data {
                header: DataHeader {
                    count: 0,
                    format: typing.format().as_data_format(),
                },
                raw: Vec::new(),
            },
            Type::List(_) => Unspecified::List(Vec::new()),
            Type::Map(_) => Unspecified::Map(BTreeMap::new()),
        }
    }

    /// Returns the type-tag ordinal for this value.
    fn type_ordinal(&self) -> u8 {
        match self {
            // Data preserves the original wire ordinal.
            Unspecified::Data { header, .. } => header.format.ordinal,
            // All other variants delegate to their Type's ordinal.
            _ => self.as_type().ordinal(),
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
            // count=0: no payload follows.
            Unspecified::Default => Ok(()),
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
            Unspecified::List(items) => encode_unspecified_list(items, writer),
            Unspecified::Map(map) => {
                // Collect keys and values for wire encoding as two sub-lists.
                let keys: Vec<Unspecified> =
                    map.keys().map(|k| Unspecified::Text(k.clone())).collect();
                let values: Vec<Unspecified> = map.values().cloned().collect();
                encode_unspecified_list(&keys, writer)?;
                encode_unspecified_list(&values, writer)?;
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
            Unspecified::Default => DataHeader::default().encode(writer),

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

            Unspecified::Text(v) => DataHeader {
                count: codec::try_count(v.len())?,
                format: DataFormat {
                    blob_size: 1,
                    data_fields: 0,
                    ordinal: self.type_ordinal(),
                },
            }
            .encode(writer),

            // List: always count=1 wrapping an inner typed header.
            Unspecified::List(_) => DataHeader {
                count: 1,
                format: DataFormat {
                    blob_size: 0,
                    data_fields: 1,
                    ordinal: self.type_ordinal(),
                },
            }
            .encode(writer),

            // Map: 2 data fields (keys list + values list).
            Unspecified::Map(_) => DataHeader {
                count: 1,
                format: DataFormat {
                    blob_size: 0,
                    data_fields: 2,
                    ordinal: self.type_ordinal(),
                },
            }
            .encode(writer),

            // Typed: preserve the original header.
            Unspecified::Data { header, .. } => header.encode(writer),
        }
    }
}

/// Encodes a slice of [`Unspecified`] values.
///
/// If all elements share the same type, encodes a homogeneous
/// inner header so per-element headers are avoided for scalar types.
///
/// Otherwise, encodes a heterogeneous inner header where each element
/// carries a self-describing header.
fn encode_unspecified_list(
    items: &[Unspecified],
    writer: &mut (impl WritesEncodable + ?Sized),
) -> Result<(), CodecError> {
    let count = codec::try_count(items.len())?;

    match Unspecified::homogeneous_element_type(items) {
        Some(element_type) => {
            // Homogeneous: inner header carries the element type.
            let inner_format = match element_type.format() {
                // Scalar blobs: elements are raw blob bytes.
                Format::Blob(blob_size) => DataFormat {
                    blob_size,
                    data_fields: 0,
                    ordinal: element_type.ordinal(),
                },
                // Structured types (Text, List, Map, Data):
                // each element still needs its own sub-header
                // for variable-length data.
                Format::Data(_) => DataFormat {
                    blob_size: 0,
                    data_fields: 1,
                    ordinal: element_type.ordinal(),
                },
                Format::Fluid => DataFormat {
                    blob_size: 0,
                    data_fields: 1,
                    ordinal: element_type.ordinal(),
                },
            };

            DataHeader {
                count,
                format: inner_format,
            }
            .encode(writer)?;

            if inner_format.data_fields == 0 {
                // Scalar: encode just the blob payload.
                for item in items {
                    item.encode(writer)?;
                }
            } else {
                // Structured: each element self-describes.
                for item in items {
                    writer.write_data(item)?;
                }
            }
        }
        None => {
            // Heterogeneous (or empty): each element self-describes.
            DataHeader {
                count,
                format: DataFormat {
                    blob_size: 0,
                    data_fields: 1,
                    ordinal: 0,
                },
            }
            .encode(writer)?;

            for item in items {
                writer.write_data(item)?;
            }
        }
    }

    Ok(())
}

/// Decodes an inner typed list from `reader`.
///
/// Reads the inner header and decodes elements according to
/// the header's ordinal:
/// - Known scalar ordinal with `data_fields=0`: decode N blobs
///   as the corresponding [`Unspecified`] variant.
/// - Known structured ordinal (Text, List, Map) with `data_fields>0`:
///   decode N self-describing elements.
/// - Ordinal 0 with `data_fields=1`: heterogeneous, each element
///   is fully self-describing.
/// - Ordinal 0 with `data_fields=0`: empty or all-Default.
fn decode_unspecified_list(
    reader: &mut (impl ReadsDecodable + ?Sized),
) -> Result<Vec<Unspecified>, CodecError> {
    let inner: DataHeader = reader.read_data()?;
    let count = inner.count as usize;
    let mut items = Vec::with_capacity(count);

    match Type::from_ordinal(inner.format.ordinal) {
        // Default list with no data fields: nothing to read.
        Some(Type::Unspecified) if inner.format.data_fields == 0 => {}

        // Homogeneous scalar types (blob, no per-element header).
        Some(Type::U8) => {
            for _ in 0..count {
                let mut v = 0u8;
                v.decode(reader, None)?;
                items.push(Unspecified::U8(v));
            }
        }
        Some(Type::U16) => {
            for _ in 0..count {
                let mut v = 0u16;
                v.decode(reader, None)?;
                items.push(Unspecified::U16(v));
            }
        }
        Some(Type::U32) => {
            for _ in 0..count {
                let mut v = 0u32;
                v.decode(reader, None)?;
                items.push(Unspecified::U32(v));
            }
        }
        Some(Type::U64) => {
            for _ in 0..count {
                let mut v = 0u64;
                v.decode(reader, None)?;
                items.push(Unspecified::U64(v));
            }
        }
        Some(Type::I8) => {
            for _ in 0..count {
                let mut v = 0i8;
                v.decode(reader, None)?;
                items.push(Unspecified::I8(v));
            }
        }
        Some(Type::I16) => {
            for _ in 0..count {
                let mut v = 0i16;
                v.decode(reader, None)?;
                items.push(Unspecified::I16(v));
            }
        }
        Some(Type::I32) => {
            for _ in 0..count {
                let mut v = 0i32;
                v.decode(reader, None)?;
                items.push(Unspecified::I32(v));
            }
        }
        Some(Type::I64) => {
            for _ in 0..count {
                let mut v = 0i64;
                v.decode(reader, None)?;
                items.push(Unspecified::I64(v));
            }
        }
        Some(Type::F32) => {
            for _ in 0..count {
                let mut v = 0.0f32;
                v.decode(reader, None)?;
                items.push(Unspecified::F32(v));
            }
        }
        Some(Type::F64) => {
            for _ in 0..count {
                let mut v = 0.0f64;
                v.decode(reader, None)?;
                items.push(Unspecified::F64(v));
            }
        }
        Some(Type::Bool) => {
            for _ in 0..count {
                let mut v = false;
                v.decode(reader, None)?;
                items.push(Unspecified::Bool(v));
            }
        }

        // Structured, heterogeneous, or unknown: each element
        // carries its own self-describing header.
        _ => {
            for _ in 0..count {
                let mut item = Unspecified::Default;
                reader.read_data_into(&mut item)?;
                items.push(item);
            }
        }
    }

    Ok(items)
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
                *self = Unspecified::Default;
                return Ok(());
            }
        };

        match Type::from_ordinal(header.format.ordinal) {
            Some(Type::Unspecified) => {
                // Skip any data that might be present.
                for _ in 0..header.count {
                    reader.skip_blob(header.format.blob_size as usize)?;
                    for _ in 0..header.format.data_fields {
                        reader.skip_data()?;
                    }
                }
                *self = Unspecified::Default;
            }

            Some(Type::U8) => {
                let mut v = 0u8;
                v.decode(reader, None)?;
                *self = Unspecified::U8(v);
            }
            Some(Type::U16) => {
                let mut v = 0u16;
                v.decode(reader, None)?;
                *self = Unspecified::U16(v);
            }
            Some(Type::U32) => {
                let mut v = 0u32;
                v.decode(reader, None)?;
                *self = Unspecified::U32(v);
            }
            Some(Type::U64) => {
                let mut v = 0u64;
                v.decode(reader, None)?;
                *self = Unspecified::U64(v);
            }
            Some(Type::I8) => {
                let mut v = 0i8;
                v.decode(reader, None)?;
                *self = Unspecified::I8(v);
            }
            Some(Type::I16) => {
                let mut v = 0i16;
                v.decode(reader, None)?;
                *self = Unspecified::I16(v);
            }
            Some(Type::I32) => {
                let mut v = 0i32;
                v.decode(reader, None)?;
                *self = Unspecified::I32(v);
            }
            Some(Type::I64) => {
                let mut v = 0i64;
                v.decode(reader, None)?;
                *self = Unspecified::I64(v);
            }
            Some(Type::F32) => {
                let mut v = 0.0f32;
                v.decode(reader, None)?;
                *self = Unspecified::F32(v);
            }
            Some(Type::F64) => {
                let mut v = 0.0f64;
                v.decode(reader, None)?;
                *self = Unspecified::F64(v);
            }
            Some(Type::Bool) => {
                let mut v = false;
                v.decode(reader, None)?;
                *self = Unspecified::Bool(v);
            }

            Some(Type::Text) => {
                // Create a copy of the original header with the
                // ordinal zeroed out, matching internal types'
                // expectation of having `ordinal = 0`.
                let header = DataHeader {
                    count: header.count,
                    format: DataFormat {
                        ordinal: 0,
                        ..header.format
                    },
                };

                let mut v = Text::default();
                v.decode(reader, Some(header))?;
                *self = Unspecified::Text(v);
            }

            Some(Type::List(_)) => {
                // Outer header has count=1, data_fields=1.
                // Read the inner header that describes element type + count.
                let items = decode_unspecified_list(reader)?;
                *self = Unspecified::List(items);
            }

            Some(Type::Map(_)) => {
                // Two data fields: keys list, values list.
                let keys_vec = decode_unspecified_list(reader)?;
                let values_vec = decode_unspecified_list(reader)?;

                // Build BTreeMap, validating that all keys are Text.
                let mut map = BTreeMap::new();
                for (key, value) in keys_vec.into_iter().zip(values_vec) {
                    match key {
                        Unspecified::Text(t) => {
                            map.insert(t, value);
                        }
                        other => {
                            return Err(CodecError::UnsupportedUnspecifiedMapKey {
                                ordinal: other.type_ordinal(),
                            })
                        }
                    }
                }
                *self = Unspecified::Map(map);
            }

            // Decode unknown ordinals as opaque data,
            // capturing all `count` entries verbatim.
            _ => {
                let mut raw = Vec::new();
                for _ in 0..header.count {
                    // Capture blob bytes.
                    if header.format.blob_size > 0 {
                        let start = raw.len();
                        raw.resize(start + header.format.blob_size as usize, 0);
                        reader.read_exact(&mut raw[start..])?;
                    }

                    // Capture data fields (header + payload) verbatim.
                    for _ in 0..header.format.data_fields {
                        capture_data(reader, &mut raw)?;
                    }
                }

                *self = Unspecified::Data { header, raw };
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
            Unspecified::Default => serializer.serialize_unit(),
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
            Unspecified::Map(map) => {
                use serde::ser::SerializeMap;
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (key, value) in map {
                    m.serialize_entry(key, value)?;
                }
                m.end()
            }
            Unspecified::Data { .. } => {
                // [`Unspecified::Data`] serializes as `null` since opaque binary
                // data has no meaningful JSON representation.
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
        Ok(Unspecified::Default)
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(Unspecified::Default)
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

    /// JSON integers are untyped, so unsigned values that fit
    /// in [`i64`] are normalized to [`Unspecified::I64`].
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
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
        let mut result = BTreeMap::new();
        while let Some((key, value)) = map.next_entry::<Text, Unspecified>()? {
            result.insert(key, value);
        }
        Ok(Unspecified::Map(result))
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

            let mut decoded = Unspecified::Default;
            let header: DataHeader = (&mut bytes.as_slice()).read_data()?;
            decoded.decode(&mut bytes.as_slice().split_at(8).1, Some(header))?;

            // Simpler: use read_data_into
            let mut decoded2 = Unspecified::Default;
            (&mut bytes.as_slice()).read_data_into(&mut decoded2)?;

            assert_eq!(*original, decoded2, "round-trip failed for {original:?}");
        }

        Ok(())
    }

    #[test]
    pub fn heterogeneous_list_round_trips() -> Result<(), CodecError> {
        let original = Unspecified::List(alloc::vec![
            Unspecified::I32(1),
            Unspecified::Text("two".into()),
            Unspecified::Bool(true),
        ]);

        let mut bytes = alloc::vec![];
        bytes.write_data(&original)?;

        let mut decoded = Unspecified::Default;
        (&mut bytes.as_slice()).read_data_into(&mut decoded)?;

        assert_eq!(original, decoded);

        Ok(())
    }

    #[test]
    pub fn homogeneous_list_round_trips() -> Result<(), CodecError> {
        // Homogeneous scalar list (all U32).
        let original = Unspecified::List(alloc::vec![
            Unspecified::U32(10),
            Unspecified::U32(20),
            Unspecified::U32(30),
        ]);

        let mut bytes = alloc::vec![];
        bytes.write_data(&original)?;

        // Verify compact encoding: outer(8) + inner(8) + 3*4 = 28 bytes.
        assert_eq!(
            28,
            bytes.len(),
            "homogeneous U32 list should be compactly encoded"
        );

        let mut decoded = Unspecified::Default;
        (&mut bytes.as_slice()).read_data_into(&mut decoded)?;

        assert_eq!(original, decoded);

        // Homogeneous text list.
        let original = Unspecified::List(alloc::vec![
            Unspecified::Text("hello".into()),
            Unspecified::Text("world".into()),
        ]);

        let mut bytes = alloc::vec![];
        bytes.write_data(&original)?;

        let mut decoded = Unspecified::Default;
        (&mut bytes.as_slice()).read_data_into(&mut decoded)?;

        assert_eq!(original, decoded);

        Ok(())
    }

    #[test]
    pub fn map_round_trips() -> Result<(), CodecError> {
        let mut map = BTreeMap::new();
        map.insert(Text::from("a"), Unspecified::I32(1));
        map.insert(Text::from("b"), Unspecified::Bool(true));
        let original = Unspecified::Map(map);

        let mut bytes = alloc::vec![];
        bytes.write_data(&original)?;

        let mut decoded = Unspecified::Default;
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
        let mut decoded = Unspecified::Default;
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
    pub fn list_with_default_round_trips() -> Result<(), CodecError> {
        let original = Unspecified::List(alloc::vec![
            Unspecified::Default,
            Unspecified::I32(42),
            Unspecified::Default,
        ]);

        let mut bytes = alloc::vec![];
        bytes.write_data(&original)?;

        let mut decoded = Unspecified::Default;
        (&mut bytes.as_slice()).read_data_into(&mut decoded)?;

        assert_eq!(original, decoded);

        Ok(())
    }

    #[test]
    pub fn default_encodes_as_zero_header() -> Result<(), CodecError> {
        let value = Unspecified::Default;
        let mut bytes = alloc::vec![];
        bytes.write_data(&value)?;
        assert_eq!(8, bytes.len(), "Default should encode as one 8-byte header");
        assert!(
            bytes.iter().all(|&b| b == 0),
            "Default header should be all zeros"
        );

        // Round-trip.
        let mut decoded = Unspecified::U8(0xFF);
        (&mut bytes.as_slice()).read_data_into(&mut decoded)?;
        assert_eq!(Unspecified::Default, decoded);

        Ok(())
    }
}
