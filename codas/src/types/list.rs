//! List data types (including `[u8]` and `Option`).

use alloc::vec::Vec;

use crate::codec::{
    CodecError, DataFormat, DataHeader, Decodable, Encodable, Format, ReadsDecodable,
    WritesEncodable,
};

impl Encodable for [u8] {
    /// Encoded as a sequence of [`Format::Data`],
    /// each containing a single [`u8`] from the slice.
    const FORMAT: Format = Format::data(0).with(Format::Blob(1));

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        writer.write_all(self)?;
        Ok(())
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        DataHeader {
            count: self.len() as u32,
            format: DataFormat {
                blob_size: 1,
                data_fields: 0,
                ordinal: 0,
            },
        }
        .encode(writer)
    }
}

impl<T> Encodable for Vec<T>
where
    T: Encodable + 'static,
{
    /// Encoded as a sequence of [`Format::Data`], each
    /// containing a single `T` from the vector.
    ///
    /// A `Vec<u8>` has the same encoding as a `[u8]`.
    const FORMAT: Format = Format::data(0).with(T::FORMAT);

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
            count: self.len() as u32,
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
        codec::{Encodable, Format, ReadsDecodable, WritesEncodable},
        types::Text,
    };

    #[test]
    fn codes_u8_slices() {
        assert_eq!(<[u8]>::FORMAT, <Vec<u8>>::FORMAT);
        assert_eq!(Format::data(0).with(Format::Blob(1)), <Vec<u8>>::FORMAT);

        let value = &[8u8, 3, 7][..];
        let mut encoded = vec![];
        encoded.write_data(value).expect("encoded");
        let decoded: Vec<u8> = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded.as_slice());
    }

    #[test]
    fn codes_unstructured_vecs() {
        assert_eq!(Format::data(0).with(Format::Blob(4)), <Vec<u32>>::FORMAT);

        let value = vec![7u32, 8, 9];
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded: Vec<u32> = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn codes_structured_vecs() {
        assert_eq!(
            Format::data(0).with(Format::data(0).with(Format::Blob(1))),
            <Vec<Text>>::FORMAT
        );

        let value = vec![Text::from("Hello, world!")];
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded: Vec<Text> = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }
}
