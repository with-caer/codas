//! List data types (including `[u8]` and `Option`).

use alloc::vec::Vec;

use crate::codec::{
    CodecError, DataFormat, DataHeader, Decodable, Encodable, Format, FormatMetadata,
    ReadsDecodable, WritesEncodable,
};

impl Encodable for [u8] {
    /// Encoded as a sequence of [`slice::len`]
    /// [`Format::Data`], each containing a
    /// single [`u8`] from the slice.
    const FORMAT: Format = u8::FORMAT.as_data_format().as_format();

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        writer.write_all(self)?;
        Ok(())
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        DataHeader {
            count: self.len() as FormatMetadata,
            format: DataFormat {
                ordinal: 0,
                blob_size: 1,
                data_fields: 0,
            },
        }
        .encode(writer)
    }
}

impl<T> Encodable for Vec<T>
where
    T: Encodable + 'static,
{
    /// Encoded as a sequence of [`Vec::len`] [`Format::Data`]
    /// containing `T`'s [`Encodable::FORMAT`].
    ///
    /// The encoding format of vectors is linked to the
    /// format of \[[`u8`]\]s: A `vec![1337u8]` and
    /// `&[1337u8]` should have identical encodings.
    const FORMAT: Format = T::FORMAT.as_data_format().as_format();

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        for item in self {
            writer.write_data(item)?;
        }

        Ok(())
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        DataHeader {
            count: self.len() as FormatMetadata,
            format: Self::FORMAT.as_data_format(),
        }
        .encode(writer)
    }
}

impl<T> Decodable for Vec<T>
where
    T: Decodable + Default + 'static,
{
    fn decode(
        &mut self,
        reader: &mut (impl ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let header = Self::ensure_header(header, &[0])?;

        // To mitigate repeat allocations, reserve
        // space for any elements in excess of this
        // vector's current capacity.
        let count = header.count as usize;
        if self.capacity() < count {
            self.reserve_exact(count - self.capacity());
        }

        // Decode all elements.
        for i in 0..count {
            let mut item = self.get_mut(i);
            if item.is_none() {
                self.push(T::default());
                item = Some(self.get_mut(i).expect("must exist"));
            }
            let item = item.unwrap();

            reader.read_data_into(item)?;
        }

        self.truncate(count);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        codec::{ReadsDecodable, WritesEncodable},
        types::Text,
    };

    #[test]
    fn codes_u8_slices() {
        let value = &[8u8, 3, 7][..];
        let mut encoded = vec![];
        encoded.write_data(value).expect("encoded");
        let decoded: Vec<u8> = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded.as_slice());
    }

    #[test]
    fn codes_unstructured_vecs() {
        let value = vec![7u32, 8, 9];
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded: Vec<u32> = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn codes_structured_vecs() {
        let value = vec![Text::from("Hello, world!")];
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded: Vec<Text> = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }
}
