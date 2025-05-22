//! Built-in data types and their in-memory
//! representations.
//!
//! # Unstable
//!
//! The APIs exposed by this module are _primarily_
//! used for code generation and dynamic data manipulation;
//! the exact APIs are subject to change, and may
//! not be well-optimized.
use core::convert::Infallible;

use alloc::{boxed::Box, vec, vec::Vec};

use crate::codec::{
    CodecError, DataFormat, DataHeader, Decodable, Encodable, Format, FormatMetadata,
    ReadsDecodable, WritesEncodable,
};

pub mod binary;
pub mod cryptography;
pub mod dynamic;
pub mod list;
pub mod map;
pub mod number;
mod text;
pub use text::*;

/// Enumeration of available built in types.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Unsigned (positive) 8-bit number.
    U8,
    /// Unsigned (positive) 16-bit number.
    U16,
    /// Unsigned (positive) 32-bit number.
    U32,
    /// Unsigned (positive) 64-bit number.
    U64,

    /// Signed (positive or negative) 8-bit number.
    I8,
    /// Signed (positive or negative) 16-bit number.
    I16,
    /// Signed (positive or negative) 32-bit number.
    I32,
    /// Signed (positive or negative) 64-bit number.
    I64,

    /// 32-bit floating point (decimal) number.
    F32,
    /// 64-bit floating point (decimal) number.
    F64,

    /// Boolean (true or false).
    Bool,

    /// UTF-8 encoded text.
    Text,

    /// Data with [`DataType`].
    Data(DataType),

    /// Data with [`Type`] that's _semantically_ a list.
    List(Box<Type>),

    /// A mapping between data of two types.
    Map(Box<(Type, Type)>),
}

impl Type {
    /// The type's encoding format.
    pub const fn format(&self) -> Format {
        match self {
            Type::U8 => u8::FORMAT,
            Type::U16 => u16::FORMAT,
            Type::U32 => u32::FORMAT,
            Type::U64 => u64::FORMAT,
            Type::I8 => i8::FORMAT,
            Type::I16 => i16::FORMAT,
            Type::I32 => i32::FORMAT,
            Type::I64 => i64::FORMAT,
            Type::F32 => f32::FORMAT,
            Type::F64 => f64::FORMAT,
            Type::Bool => bool::FORMAT,
            Type::Text => Text::FORMAT,
            Type::Data(data) => data.format,
            Type::List(typing) => typing.format().as_data_format().as_format(),

            // Maps are formatted as a list of keys
            // followed by a list of values.
            Type::Map(..) => DataFormat {
                ordinal: 0,
                blob_size: 0,
                data_fields: 2,
            }
            .as_format(),
        }
    }

    /// Returns the type with `name`.
    ///
    /// This function assumes `name` is in ASCII lowercase.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "u8" => Some(Type::U8),
            "u16" => Some(Type::U16),
            "u32" => Some(Type::U32),
            "u64" => Some(Type::U64),
            "i8" => Some(Type::I8),
            "i16" => Some(Type::I16),
            "i32" => Some(Type::I32),
            "i64" => Some(Type::I64),
            "f32" => Some(Type::F32),
            "f64" => Some(Type::F64),
            "bool" => Some(Type::Bool),
            "text" => Some(Type::Text),
            _ => None,
        }
    }
}

impl Default for Type {
    fn default() -> Self {
        Self::Data(Unspecified::DATA_TYPE)
    }
}

/// In-memory representation of a coda.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Coda {
    /// The coda's full name, including any
    /// hierarchical components and separators.
    pub global_name: Text,

    /// The final component of [`Self::global_name`]
    /// that does not describe a hierarchy.
    pub local_name: Text,

    pub docs: Option<Text>,

    /// Data in ascending order by ordinal.
    pub(crate) data: Vec<DataType>,
}

impl Coda {
    /// Returns a new coda containing `data`.
    pub fn new(global_name: Text, local_name: Text, docs: Option<Text>, data: &[DataType]) -> Self {
        Self {
            global_name,
            local_name,
            docs,
            data: Vec::from(data),
        }
    }

    /// Returns an iterator over all data types in the coda.
    ///
    /// The implicit [`crate::types::Unspecified`] data type
    /// is _not_ included in the returned iterator.
    pub fn iter(&self) -> impl Iterator<Item = &DataType> {
        self.data.iter()
    }

    /// Returns the data type with `name`,
    /// if it is known by the coda.
    #[cfg(feature = "parse")]
    pub(crate) fn type_from_name(&self, name: &str) -> Option<Type> {
        for data in self.data.iter() {
            if data.name.eq_ignore_ascii_case(name) {
                return Some(Type::Data(data.clone()));
            }
        }

        Type::from_name(name)
    }
}

/// Data containing a structured set of [`DataField`]s.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct DataType {
    /// The name of the data type.
    ///
    /// TODO: We've been structuring names similar
    /// to fully-qualified Rust type names (like `my::data::TypeName`).
    /// We should standardize on a language-neutral naming
    /// convention; perhaps HTTP-style URLs (like `/my/data/TypeName`)
    /// so downstream tools have an easy way to map hierarchical
    /// names back to native type names as appropriate.
    pub name: Text,

    /// Markdown-formatted documentation of the data type.
    pub docs: Option<Text>,

    /// Ordered set of [`Format::Blob`]
    /// fields in the data type.
    blob_fields: Vec<DataField>,

    /// Ordered set of [`Format::Data`]
    /// fields in the data type.
    ///
    /// These fields are always encoded, in
    /// order, _after_ all [`Self::blob_fields`].
    data_fields: Vec<DataField>,

    /// The encoding format of data with this type.
    format: Format,
}

impl DataType {
    /// Returns a new fixed data type with
    /// `name`, `ordinal`, `blob_fields`, and `data_fields`.
    pub fn new(
        name: Text,
        docs: Option<Text>,
        ordinal: FormatMetadata,
        blob_fields: &[DataField],
        data_fields: &[DataField],
    ) -> Self {
        // Build a new encoding format for the data.
        let mut format = Format::data(ordinal);

        // Add blob fields to the format.
        let mut i = 0;
        while i < blob_fields.len() {
            let field = &blob_fields[i];
            format = format.with(field.typing.format());
            i += 1;
        }

        // Add data fields to the format.
        let mut i = 0;
        while i < data_fields.len() {
            let field = &data_fields[i];
            format = format.with(field.typing.format());
            i += 1;
        }

        Self {
            name,
            docs,
            blob_fields: Vec::from(blob_fields),
            data_fields: Vec::from(data_fields),
            format,
        }
    }

    /// Returns a new data type with a fluid format.
    pub const fn new_fluid(name: Text, docs: Option<Text>) -> Self {
        Self {
            name,
            docs,
            blob_fields: vec![],
            data_fields: vec![],
            format: Format::Fluid,
        }
    }

    /// Returns an iterator over all fields within the type.
    pub fn iter(&self) -> impl Iterator<Item = &DataField> {
        self.blob_fields.iter().chain(self.data_fields.iter())
    }

    /// Adds a new `field` to the type.
    pub fn with(mut self, field: DataField) -> Self {
        if matches!(self.format, Format::Fluid) {
            todo!("it should be an error to add fields to a type defined as fluid")
        }

        let field_format = field.typing.format();
        self.format = self.format.with(field_format);
        match field_format {
            Format::Blob(..) => {
                self.blob_fields.push(field);
            }
            Format::Data(..) | Format::Fluid => {
                self.data_fields.push(field);
            }
        };

        self
    }

    /// Returns the type's encoding format.
    pub const fn format(&self) -> &Format {
        &self.format
    }
}

/// A field in a [`DataType`].
#[derive(Default, Clone, Debug, PartialEq)]
pub struct DataField {
    /// Name of the field.
    pub name: Text,

    /// Markdown-formatted documentation of the field.
    pub docs: Option<Text>,

    /// Type of the field.
    pub typing: Type,

    /// True if the field is semantically optional.
    pub optional: bool,

    /// True if the field is semantically flattened.
    ///
    /// This property has _no_ effect on the encoding,
    /// decoding, or in-language representation of
    /// a field; it's an informational marker that some
    /// marshallers (like JSON) may use to enable
    /// compatibility between coda-defined data and
    /// legacy systems.
    pub flattened: bool,
}

/// Unspecified data.
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Unspecified {}

impl Unspecified {
    /// Constant [`DataType`] for unspecified data.
    pub const DATA_TYPE: DataType = DataType::new_fluid(
        Text::from("Unspecified"),
        Some(Text::from("Unspecified data.")),
    );
}

/// A thing that _might_ contain data with a
/// specific format `D`.
///
/// This trait is mainly intended for use with the
/// enums auto-generated for [`Coda`]s
pub trait TryAsFormat<D> {
    /// Type of error returned when `self`
    /// doesn't contain data of format `D`.
    ///
    /// This error should be the ordinal
    /// identifier of the _actual_ data in `D`,
    /// or [`Infallible`].
    type Error;

    /// Returns a `D`-formatted reference to the data.
    fn try_as_format(&self) -> Result<&D, Self::Error>;
}

/// Every data format can be interpreted as itself.
impl<T> TryAsFormat<T> for T {
    type Error = Infallible;

    fn try_as_format(&self) -> Result<&T, Self::Error> {
        Ok(self)
    }
}

// Codecs /////////////////////////////////////////////////

impl Encodable for Type {
    const FORMAT: Format = Format::Fluid;

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        match self {
            Type::Data(typing) => writer.write_data(typing),
            Type::List(typing) => writer.write_data(typing.as_ref()),
            Type::Map(typing) => {
                writer.write_data(&typing.as_ref().0)?;
                writer.write_data(&typing.as_ref().1)?;
                Ok(())
            }

            // Only data types contain additional encoded info.
            _ => Ok(()),
        }
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        let ordinal = match self {
            Type::U8 => 1u16,
            Type::U16 => 2u16,
            Type::U32 => 3u16,
            Type::U64 => 4u16,
            Type::I8 => 5u16,
            Type::I16 => 6u16,
            Type::I32 => 7u16,
            Type::I64 => 8u16,
            Type::F32 => 9u16,
            Type::F64 => 10u16,
            Type::Bool => 11u16,
            Type::Text => 12u16,
            Type::Data(..) => {
                return DataHeader {
                    count: 1,
                    format: Format::data(13u16).with(Type::FORMAT).as_data_format(),
                }
                .encode(writer);
            }
            Type::List { .. } => {
                return DataHeader {
                    count: 1,
                    format: Format::data(14u16).with(Type::FORMAT).as_data_format(),
                }
                .encode(writer);
            }
            Type::Map { .. } => {
                return DataHeader {
                    count: 1,
                    format: Format::data(15u16).with(Type::FORMAT).as_data_format(),
                }
                .encode(writer);
            }
        };

        DataHeader {
            count: 1,
            format: Format::data(ordinal).as_data_format(),
        }
        .encode(writer)
    }
}

impl Decodable for Type {
    fn decode(
        &mut self,
        reader: &mut (impl ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let header = Self::ensure_header(
            header,
            &[
                1u16, 2u16, 3u16, 4u16, 5u16, 6u16, 7u16, 8u16, 9u16, 10u16, 11u16, 12u16, 13u16,
                14u16, 15u16,
            ],
        )?;

        match header.format.ordinal {
            1u16 => {
                *self = Type::U8;
            }
            2u16 => {
                *self = Type::U16;
            }
            3u16 => {
                *self = Type::U32;
            }
            4u16 => {
                *self = Type::U64;
            }
            5u16 => {
                *self = Type::I8;
            }
            6u16 => {
                *self = Type::I16;
            }
            7u16 => {
                *self = Type::I32;
            }
            8u16 => {
                *self = Type::I64;
            }
            9u16 => {
                *self = Type::F32;
            }
            10u16 => {
                *self = Type::F64;
            }
            11u16 => {
                *self = Type::Bool;
            }
            12u16 => {
                *self = Type::Text;
            }
            13u16 => {
                let mut typing = DataType::default();
                reader.read_data_into(&mut typing)?;
                *self = Type::Data(typing);
            }
            14u16 => {
                let mut typing = Type::default();
                reader.read_data_into(&mut typing)?;
                *self = Type::List(typing.into());
            }
            15u16 => {
                let mut key_typing = Type::default();
                reader.read_data_into(&mut key_typing)?;
                let mut value_typing = Type::default();
                reader.read_data_into(&mut value_typing)?;
                *self = Type::Map((key_typing, value_typing).into());
            }
            _ => unreachable!(),
        };

        Ok(())
    }
}

impl Encodable for Coda {
    const FORMAT: crate::codec::Format = Format::data(0)
        .with(Text::FORMAT)
        .with(Text::FORMAT)
        .with(Text::FORMAT)
        .with(Vec::<DataType>::FORMAT);

    fn encode(
        &self,
        writer: &mut (impl crate::codec::WritesEncodable + ?Sized),
    ) -> Result<(), crate::codec::CodecError> {
        writer.write_data(&self.global_name)?;
        writer.write_data(&self.local_name)?;
        writer.write_data(&self.docs)?;
        writer.write_data(&self.data)?;
        Ok(())
    }
}

impl Decodable for Coda {
    fn decode(
        &mut self,
        reader: &mut (impl crate::codec::ReadsDecodable + ?Sized),
        header: Option<crate::codec::DataHeader>,
    ) -> Result<(), crate::codec::CodecError> {
        let _ = Self::ensure_header(header, &[0u16])?;

        reader.read_data_into(&mut self.global_name)?;
        reader.read_data_into(&mut self.local_name)?;
        reader.read_data_into(&mut self.docs)?;
        reader.read_data_into(&mut self.data)?;

        Ok(())
    }
}

impl Encodable for DataType {
    const FORMAT: Format = Format::data(0)
        .with(Text::FORMAT)
        .with(Option::<Text>::FORMAT)
        .with(Vec::<DataField>::FORMAT)
        .with(Vec::<DataField>::FORMAT)
        .with(Format::FORMAT);

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        writer.write_data(&self.name)?;
        writer.write_data(&self.docs)?;
        writer.write_data(&self.blob_fields)?;
        writer.write_data(&self.data_fields)?;
        writer.write_data(&self.format)?;
        Ok(())
    }
}

impl Decodable for DataType {
    fn decode(
        &mut self,
        reader: &mut (impl ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let _ = Self::ensure_header(header, &[0])?;

        reader.read_data_into(&mut self.name)?;
        reader.read_data_into(&mut self.docs)?;
        reader.read_data_into(&mut self.blob_fields)?;
        reader.read_data_into(&mut self.data_fields)?;
        reader.read_data_into(&mut self.format)?;

        Ok(())
    }
}

impl Encodable for DataField {
    const FORMAT: Format = Format::data(0)
        .with(bool::FORMAT)
        .with(bool::FORMAT)
        .with(Text::FORMAT)
        .with(Option::<Text>::FORMAT)
        .with(Type::FORMAT);

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        writer.write_data(&self.optional)?;
        writer.write_data(&self.flattened)?;
        writer.write_data(&self.name)?;
        writer.write_data(&self.docs)?;
        writer.write_data(&self.typing)?;
        Ok(())
    }
}

impl Decodable for DataField {
    fn decode(
        &mut self,
        reader: &mut (impl ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let _ = Self::ensure_header(header, &[0])?;
        reader.read_data_into(&mut self.optional)?;
        reader.read_data_into(&mut self.flattened)?;
        reader.read_data_into(&mut self.name)?;
        reader.read_data_into(&mut self.docs)?;
        reader.read_data_into(&mut self.typing)?;
        Ok(())
    }
}

impl Encodable for Unspecified {
    /// Surprise! The encoding format of unspecified
    /// data is unspecified (i.e., [`Format::Fluid`]).
    const FORMAT: Format = Format::Fluid;

    fn encode(&self, _writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        Ok(())
    }
}

impl Decodable for Unspecified {
    fn decode(
        &mut self,
        _reader: &mut (impl ReadsDecodable + ?Sized),
        _header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        Ok(())
    }
}

impl<T> Encodable for Option<T>
where
    T: Default + Encodable + 'static,
{
    /// Options are a semantic feature that's not
    /// technically encoded: Optional data encodes
    /// identically to "not optional" data. In the
    /// case of `None`, data is encoded and decoded
    /// as its default value.
    const FORMAT: Format = T::FORMAT;

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        match &self {
            Some(value) => {
                value.encode(writer)?;
            }

            None => {
                Self::FORMAT.encode_default_value(writer)?;
            }
        }

        Ok(())
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        match &self {
            Some(value) => value.encode_header(writer),
            None => Self::FORMAT.encode_default_header(writer),
        }
    }
}

impl<T> Decodable for Option<T>
where
    T: Decodable + Default + PartialEq + 'static,
{
    fn decode(
        &mut self,
        reader: &mut (impl ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let mut decoded = T::default();
        decoded.decode(reader, header)?;

        // TODO: It'd be better if we could detect "defaulty-ness"
        //       during the decoding step, so that we're not having
        //       to manually compare a decoded value to its default.
        if decoded == T::default() {
            *self = None;
        } else {
            *self = Some(decoded);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::codec::{Decodable, WritesEncodable};

    use super::*;

    /// Sample data structure for testing type manipulation APIs.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct TestData {
        pub number: i32,
        pub floaty: f64,
        pub text_list: Vec<Text>,
        pub text: Text,
        pub nested: NestedTestData,
        pub two_d: Vec<Vec<Text>>,
    }

    impl TestData {
        pub fn typing() -> DataType {
            let blob_fields = vec![
                DataField {
                    name: Text::from("number"),
                    docs: None,
                    typing: Type::I32,
                    optional: false,
                    flattened: false,
                },
                DataField {
                    name: Text::from("floaty"),
                    docs: None,
                    typing: Type::F64,
                    optional: false,
                    flattened: false,
                },
            ];

            let data_fields = vec![
                DataField {
                    name: Text::from("text_list"),
                    docs: None,
                    typing: Type::List(Type::Text.into()),
                    optional: false,
                    flattened: false,
                },
                DataField {
                    name: Text::from("text"),
                    docs: None,
                    typing: Type::Text,
                    optional: false,
                    flattened: false,
                },
                DataField {
                    name: Text::from("nested"),
                    docs: None,
                    typing: Type::Data(NestedTestData::typing()),
                    optional: false,
                    flattened: false,
                },
                DataField {
                    name: Text::from("two_d"),
                    docs: None,
                    typing: Type::List(Type::List(Type::Text.into()).into()),
                    optional: false,
                    flattened: false,
                },
            ];

            let typing = DataType::new(Text::from("Testdata"), None, 1, &blob_fields, &data_fields);

            assert_eq!(Self::FORMAT, *typing.format());

            typing
        }
    }

    impl Encodable for TestData {
        const FORMAT: Format = Format::data(1)
            .with(i32::FORMAT)
            .with(f64::FORMAT)
            .with(Vec::<Text>::FORMAT)
            .with(Text::FORMAT)
            .with(NestedTestData::FORMAT)
            .with(Vec::<Vec<Text>>::FORMAT);

        fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
            writer.write_data(&self.number)?;
            writer.write_data(&self.floaty)?;
            writer.write_data(&self.text_list)?;
            writer.write_data(&self.text)?;
            writer.write_data(&self.nested)?;
            writer.write_data(&self.two_d)?;
            Ok(())
        }
    }

    impl Decodable for TestData {
        fn decode(
            &mut self,
            reader: &mut (impl ReadsDecodable + ?Sized),
            header: Option<DataHeader>,
        ) -> Result<(), CodecError> {
            let _ = Self::ensure_header(header, &[1])?;

            reader.read_data_into(&mut self.number)?;
            reader.read_data_into(&mut self.floaty)?;
            reader.read_data_into(&mut self.text_list)?;
            reader.read_data_into(&mut self.text)?;
            reader.read_data_into(&mut self.nested)?;
            reader.read_data_into(&mut self.two_d)?;

            Ok(())
        }
    }

    /// Simple data structure intended for nesting
    /// inside of a [`TestData`].
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct NestedTestData {
        pub boolean: bool,
    }

    impl NestedTestData {
        pub fn typing() -> DataType {
            let blob_fields = vec![DataField {
                name: Text::from("boolean"),
                docs: None,
                typing: Type::Bool,
                optional: false,
                flattened: false,
            }];

            let data_fields = vec![];

            let typing = DataType::new(
                Text::from("NestedTestdata"),
                None,
                2,
                &blob_fields,
                &data_fields,
            );

            assert_eq!(Self::FORMAT, *typing.format());

            typing
        }
    }

    impl Encodable for NestedTestData {
        const FORMAT: Format = Format::data(2).with(bool::FORMAT);

        fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
            writer.write_data(&self.boolean)?;
            Ok(())
        }
    }

    impl Decodable for NestedTestData {
        fn decode(
            &mut self,
            reader: &mut (impl ReadsDecodable + ?Sized),
            header: Option<DataHeader>,
        ) -> Result<(), CodecError> {
            let _ = Self::ensure_header(header, &[2])?;

            reader.read_data_into(&mut self.boolean)?;

            Ok(())
        }
    }

    #[test]
    pub fn data_type_codec() {
        let data_type = TestData::typing();

        let mut encoded_data_type = vec![];
        encoded_data_type.write_data(&data_type).unwrap();
        let decoded_data_type = encoded_data_type.as_slice().read_data().unwrap();

        assert_eq!(data_type, decoded_data_type);
    }

    #[test]
    fn codes_unstructured_optionals() {
        let option: Option<u32> = Some(1337u32);
        let mut data = vec![];
        data.write_data(&option).expect("encoded");
        println!("encoded");
        let decoded_option = data.as_slice().read_data().expect("decoded");
        assert_eq!(option, decoded_option);

        // Do None values decode as None?
        let option: Option<u32> = None;
        let mut data = vec![];
        data.write_data(&option).expect("encoded");
        let decoded_option = data.as_slice().read_data().expect("decoded");
        assert_eq!(option, decoded_option);

        // Do default values decode as None?
        let option: Option<u32> = Some(0);
        let mut data = vec![];
        data.write_data(&option).expect("encoded");
        let decoded_option: Option<u32> = data.as_slice().read_data().expect("decoded");
        assert_eq!(None, decoded_option);
    }

    #[test]
    fn codes_structured_optionals() {
        let option: Option<Text> = Some("Hello, World!".into());
        let mut data = vec![];
        data.write_data(&option).expect("encoded");
        println!("encoded");
        let decoded_option = data.as_slice().read_data().expect("decoded");
        assert_eq!(option, decoded_option);

        // Do None values decode as None?
        let option: Option<Text> = None;
        let mut data = vec![];
        data.write_data(&option).expect("encoded");
        let decoded_option = data.as_slice().read_data().expect("decoded");
        assert_eq!(option, decoded_option);

        // Do default values decode as None?
        let option: Option<Text> = Some("".into());
        let mut data = vec![];
        data.write_data(&option).expect("encoded");
        let decoded_option: Option<Text> = data.as_slice().read_data().expect("decoded");
        assert_eq!(None, decoded_option);
    }
}
