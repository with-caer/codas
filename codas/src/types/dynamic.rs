//! ## Unstable
use alloc::{borrow::ToOwned, collections::BTreeMap, sync::Arc};

use crate::{
    codec::{
        CodecError, DataHeader, Decodable, Encodable, Format, FormatMetadata, WritesEncodable,
    },
    types::{DataField, DataType, Type},
};

use super::Text;

/// Dynamic value of some [`Type`].
#[derive(Debug, Clone, PartialEq)]
pub enum Dynamic {
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

    /// Set of named dynamic values (i.e., fields).
    Data(DynamicDataValue),

    /// List of dynamic values.
    List(DynamicListValue),

    /// Mapping of dynamic values.
    Map(DynamicMapValue),
}

impl Dynamic {
    /// Returns the default value of a `typing`.
    pub fn default(typing: &Type) -> Dynamic {
        match typing {
            Type::U8 => Dynamic::U8(0),
            Type::I8 => Dynamic::I8(0),
            Type::U16 => Dynamic::U16(0),
            Type::I16 => Dynamic::I16(0),
            Type::U32 => Dynamic::U32(0),
            Type::I32 => Dynamic::I32(0),
            Type::U64 => Dynamic::U64(0),
            Type::I64 => Dynamic::I64(0),
            Type::F32 => Dynamic::F32(0.0),
            Type::F64 => Dynamic::F64(0.0),
            Type::Bool => Dynamic::Bool(false),
            Type::Text => Dynamic::Text(Text::default()),
            Type::Data(typing) => Dynamic::Data(DynamicDataValue::new(typing)),
            Type::List(typing) => Dynamic::List(DynamicListValue::new(typing)),
            Type::Map(typing) => Dynamic::Map(DynamicMapValue::new(typing)),
        }
    }
}

/// Contents of a [`Dynamic::Data`].
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicDataValue {
    typing: Arc<DataType>,
    fields: Option<BTreeMap<Text, Dynamic>>,
}

impl DynamicDataValue {
    /// Returns a new, default data value of `typing`.
    pub fn new(typing: &DataType) -> Self {
        Self {
            typing: Arc::new(typing.to_owned()),
            fields: None,
        }
    }

    /// Removes all values from this data,
    /// resetting them to their default values.
    pub fn reset(&mut self) {
        if let Some(fields) = self.fields.as_mut() {
            fields.clear();
        }
    }

    /// Inserts a `value` for the field with `name`.
    pub fn insert(&mut self, name: Text, value: Dynamic) {
        let fields = self.fields.get_or_insert_with(Default::default);
        fields.insert(name, value);
    }

    /// Returns an iterator over all fields in the data.
    ///
    /// The iterator yields fields in order by ordinal,
    /// yielding `None` for unset fields.
    pub fn iter(&self) -> impl Iterator<Item = (&DataField, Option<&Dynamic>)> {
        self.typing
            .iter()
            .map(|field| (field, self.fields.as_ref().and_then(|f| f.get(&field.name))))
    }

    /// Applies `proc` to each field in the data.
    ///
    /// Fields are visited in order by ordinal. If
    /// a field is unset, it will be initialized to
    /// a default value before `proc` is invoked.
    pub fn visit_mut(&mut self, mut proc: impl FnMut(&DataField, &mut Dynamic)) {
        let fields = self.fields.get_or_insert_with(Default::default);
        for field in self.typing.iter() {
            let value = fields
                .entry(field.name.clone())
                .or_insert_with(|| Dynamic::default(&field.typing));

            proc(field, value);
        }
    }
}

/// Contents of a [`Dynamic::List`].
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicListValue {
    typing: Arc<Type>,
    values: alloc::vec::Vec<Dynamic>,
}

impl DynamicListValue {
    /// Returns a new, empty list of values with `typing`.
    pub fn new(typing: &Type) -> Self {
        Self {
            typing: Arc::new(typing.to_owned()),
            values: alloc::vec::Vec::new(),
        }
    }

    /// Removes all values from the list.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Adds a new value to the list.
    pub fn push(&mut self, value: Dynamic) {
        self.values.push(value);
    }

    /// Returns the number of values in the list.
    pub fn len(&self) -> FormatMetadata {
        self.values.len() as FormatMetadata
    }

    /// Returns true iff the list is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns an iterator over all values in the list.
    pub fn iter(&self) -> impl Iterator<Item = &Dynamic> {
        self.values.iter()
    }

    /// Returns the typing of the values in the list.
    pub fn item_typing(&self) -> &Type {
        &self.typing
    }
}

/// Contents of a [`Dynamic::Map`].
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMapValue {
    keys: DynamicListValue,
    values: DynamicListValue,
}

impl DynamicMapValue {
    /// Returns a new, empty map with `typing`.
    pub fn new(typing: &(Type, Type)) -> Self {
        Self {
            keys: DynamicListValue::new(&typing.0),
            values: DynamicListValue::new(&typing.1),
        }
    }
}

// Encoders ///////////////////////////////////////////////
impl Encodable for Dynamic {
    const FORMAT: Format = Format::Fluid;

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        macros::match_values!(self, v, v.encode(writer))
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        macros::match_values!(self, v, v.encode_header(writer))
    }
}

impl Encodable for DynamicDataValue {
    const FORMAT: Format = Format::Fluid;

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        // No-op if no fields are set.
        if self.fields.is_none() {
            return Ok(());
        }
        let fields = self.fields.as_ref().unwrap();

        // Encode all fields in order.
        for field in self.typing.iter() {
            if let Some(value) = fields.get(&field.name) {
                writer.write_data(value)?;
            } else {
                field.typing.format().encode_default_header(writer)?;
                field.typing.format().encode_default_value(writer)?;
            }
        }

        Ok(())
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        let count = if self.fields.is_some() { 1 } else { 0 };

        DataHeader {
            count,
            format: self.typing.format().as_data_format(),
        }
        .encode(writer)
    }
}

impl Encodable for DynamicListValue {
    const FORMAT: Format = Format::Fluid;

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        for value in &self.values {
            writer.write_data(value)?;
        }

        Ok(())
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        let count = self.values.len() as FormatMetadata;

        // Apply the same formatting rules as the Vec codec.
        let format = Format::data(0).with(self.typing.format()).as_data_format();
        DataHeader { count, format }.encode(writer)
    }
}

impl Encodable for DynamicMapValue {
    const FORMAT: Format = Format::data(0)
        .with(DynamicListValue::FORMAT)
        .with(DynamicListValue::FORMAT);

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        writer.write_data(&self.keys)?;
        writer.write_data(&self.values)?;
        Ok(())
    }
}

// Decoders ///////////////////////////////////////////////
impl Decodable for Dynamic {
    fn decode(
        &mut self,
        reader: &mut (impl crate::codec::ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        macros::match_values!(self, v, v.decode(reader, header))
    }
}

impl Decodable for DynamicDataValue {
    fn decode(
        &mut self,
        reader: &mut (impl crate::codec::ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        // FIXME: Handle other data types in the same coda.
        let header = Self::ensure_header(header, &[self.typing.format().as_data_format().ordinal])?;

        // FIXME: Skip all but the last item.
        if header.count > 1 {
            for _ in 0..header.count - 1 {
                reader.skip_data_with_format(header.format)?;
            }
        }

        // Clear any existing fields.
        let fields = self.fields.get_or_insert(Default::default());
        fields.clear();

        // Track how much of the data we've decoded so
        // we can skip any unsupported data.
        let mut remaining_blob = header.format.blob_size;
        let mut remaining_fields = header.format.data_fields;

        // Decode all fields in order.
        for field in self.typing.iter() {
            let field_format = field.typing.format();

            // Update trackers.
            if field_format.is_structured() {
                // If we encounter structured data with blob
                // data still remaining, skip the remaining blob.
                if remaining_blob > 0 {
                    reader.skip_blob(remaining_blob as usize)?;
                    remaining_blob = 0;
                }

                remaining_fields = remaining_fields
                    .checked_sub(1)
                    .ok_or(CodecError::MissingDataFields { count: 1 })?;
            } else {
                let blob_size = field_format.as_data_format().blob_size;
                remaining_blob = remaining_blob
                    .checked_sub(blob_size)
                    .ok_or(CodecError::MissingBlobLength { length: blob_size })?;
            }

            // Decode the data.
            let mut value = Dynamic::default(&field.typing);
            if field_format.is_structured() {
                let header = reader.read_data()?;
                value.decode(reader, Some(header))?;
            } else {
                value.decode(reader, None)?;
            }

            fields.insert(field.name.clone(), value);
        }

        // Skip any remaining blob data.
        if remaining_blob != 0 {
            reader.skip_blob(remaining_blob as usize)?;
        }

        // Skip any remaining data fields.
        for _ in 0..remaining_fields {
            reader.skip_data()?;
        }

        Ok(())
    }
}

impl Decodable for DynamicListValue {
    fn decode(
        &mut self,
        reader: &mut (impl crate::codec::ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let header = Self::ensure_header(header, &[0])?;

        // To mitigate repeat allocations, reserve
        // space for any elements in excess of this
        // vector's current capacity.
        let count = header.count as usize;
        if self.values.capacity() < count {
            self.values.reserve_exact(count - self.values.capacity());
        }
        self.values.clear();

        // Decode all elements.
        let value = Dynamic::default(&self.typing);
        for _ in 0..count {
            let mut value = value.clone();
            if self.typing.format().is_structured() {
                let header = reader.read_data()?;
                value.decode(reader, Some(header))?;
            } else {
                value.decode(reader, None)?;
            }
            self.values.push(value);
        }

        Ok(())
    }
}

impl Decodable for DynamicMapValue {
    fn decode(
        &mut self,
        reader: &mut (impl crate::codec::ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let _ = Self::ensure_header(header, &[0])?;

        reader.read_data_into(&mut self.keys)?;
        reader.read_data_into(&mut self.values)?;

        Ok(())
    }
}

mod macros {
    /// Macro which generates match expressions
    /// for all possible types of [`Value`].
    macro_rules! match_values {
        (
            $enum_var:ident,
            $value_var:ident,
            $value_expr:expr
        ) => {
            match $enum_var {
                Dynamic::U8($value_var) => $value_expr,
                Dynamic::I8($value_var) => $value_expr,
                Dynamic::U16($value_var) => $value_expr,
                Dynamic::I16($value_var) => $value_expr,
                Dynamic::U32($value_var) => $value_expr,
                Dynamic::I32($value_var) => $value_expr,
                Dynamic::U64($value_var) => $value_expr,
                Dynamic::I64($value_var) => $value_expr,
                Dynamic::F32($value_var) => $value_expr,
                Dynamic::F64($value_var) => $value_expr,
                Dynamic::Bool($value_var) => $value_expr,
                Dynamic::Text($value_var) => $value_expr,
                Dynamic::Data($value_var) => $value_expr,
                Dynamic::List($value_var) => $value_expr,
                Dynamic::Map($value_var) => $value_expr,
            }
        };
    }

    // Re-export macros for use in outer module.
    pub(crate) use match_values;
}

#[cfg(test)]
mod tests {
    use crate::codec::ReadsDecodable;

    use super::super::tests::{NestedTestData, TestData};

    use super::*;

    #[test]
    pub fn dynamic_codes() -> Result<(), CodecError> {
        let test_data_type = TestData::typing();

        // Create some test data using non-dynamic APIs.
        let test_data_static = TestData {
            number: 1,
            floaty: 60.90,
            text_list: vec!["one".into(), "two".into()],
            text: "hello".into(),
            nested: NestedTestData { boolean: true },
            two_d: vec![
                vec!["three".into(), "four".into()],
                vec!["five".into(), "six".into()],
            ],
        };
        let mut test_bytes_static = vec![];
        test_bytes_static.write_data(&test_data_static)?;

        // Create some test data using dynamic APIs.
        let mut test_data_dynamic = DynamicDataValue::new(&test_data_type);
        test_data_dynamic.insert("number".into(), Dynamic::I32(1));
        test_data_dynamic.insert("floaty".into(), Dynamic::F64(60.90));
        let mut test_data_dynamic_list = DynamicListValue::new(&Type::Text);
        test_data_dynamic_list.push(Dynamic::Text("one".into()));
        test_data_dynamic_list.push(Dynamic::Text("two".into()));
        test_data_dynamic.insert("text_list".into(), Dynamic::List(test_data_dynamic_list));
        test_data_dynamic.insert("text".into(), Dynamic::Text("hello".into()));
        let mut test_data_dynamic_nested = DynamicDataValue::new(&NestedTestData::typing());
        test_data_dynamic_nested.insert("boolean".into(), Dynamic::Bool(true));
        test_data_dynamic.insert("nested".into(), Dynamic::Data(test_data_dynamic_nested));
        let mut test_data_dynamic_two_d = DynamicListValue::new(&Type::List(Type::Text.into()));
        let mut test_data_dynamic_list_a = DynamicListValue::new(&Type::Text);
        test_data_dynamic_list_a.push(Dynamic::Text("three".into()));
        test_data_dynamic_list_a.push(Dynamic::Text("four".into()));
        test_data_dynamic_two_d.push(Dynamic::List(test_data_dynamic_list_a));
        let mut test_data_dynamic_list_b = DynamicListValue::new(&Type::Text);
        test_data_dynamic_list_b.push(Dynamic::Text("five".into()));
        test_data_dynamic_list_b.push(Dynamic::Text("six".into()));
        test_data_dynamic_two_d.push(Dynamic::List(test_data_dynamic_list_b));
        test_data_dynamic.insert("two_d".into(), Dynamic::List(test_data_dynamic_two_d));
        let mut test_bytes_dynamic = vec![];
        test_bytes_dynamic.write_data(&test_data_dynamic)?;

        // The two datas' format headers should be identical.
        let mut test_data_static_header = vec![];
        test_data_static.encode_header(&mut test_data_static_header)?;
        let mut test_data_dynamic_header = vec![];
        test_data_dynamic.encode_header(&mut test_data_dynamic_header)?;
        assert_eq!(test_data_static_header, test_data_dynamic_header);

        // The two encoded sets of bytes should be identical.
        assert_eq!(test_bytes_static, test_bytes_dynamic);

        // Check that the dynamic data decodes into static data correctly.
        let static_from_dynamic = test_bytes_dynamic.as_slice().read_data()?;
        assert_eq!(test_data_static, static_from_dynamic);

        // Check that the static data decodes into dynamic data correctly.
        let mut dynamic_from_static = DynamicDataValue::new(&test_data_type);
        (&mut test_bytes_static.as_slice()).read_data_into(&mut dynamic_from_static)?;
        assert_eq!(test_data_dynamic, dynamic_from_static);

        Ok(())
    }
}
