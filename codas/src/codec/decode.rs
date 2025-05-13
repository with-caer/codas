//! Codec decoder implementations.
use snafu::ensure;

use crate::{codec::UnsupportedDataFormatSnafu, stream::Reads};

use super::{
    encode::Encodable, CodecError, DataFormat, DataHeader, FormatMetadata,
    UnexpectedDataFormatSnafu,
};

/// Default size used for temporary,
/// stack-allocated buffers.
pub const TEMP_BUFFER_SIZE: usize = 1024;

/// A thing that decodes from
/// [`codec`](super)-compliant data.
pub trait Decodable: Encodable {
    /// Decodes data with `header` from `reader` into this thing.
    ///
    /// In most cases, [`ReadsDecodable::read_data`] or
    /// [`ReadsDecodable::read_data_into`] should be used
    /// instead of calling this function directly.
    ///
    /// If `Self`'s [`Encodable::FORMAT`] is not
    /// [`structured`](`crate::codec::Format::is_structured`),
    /// `header` will be `None`, and this function
    /// should decode a number of bytes equal to
    /// it's blob size.
    fn decode(
        &mut self,
        reader: &mut (impl ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError>;

    /// Returns `Ok(header)` iff `header` exists
    /// and matches one of `suppported_ordinals`.
    #[inline(always)]
    fn ensure_header(
        header: Option<DataHeader>,
        supported_ordinals: &[FormatMetadata],
    ) -> Result<DataHeader, CodecError> {
        // Extract header data.
        let header = header.ok_or_else(|| {
            UnexpectedDataFormatSnafu {
                expected: Self::FORMAT,
                actual: header,
            }
            .build()
        })?;

        // Validate ordinals.
        ensure!(
            supported_ordinals.contains(&header.format.ordinal),
            UnsupportedDataFormatSnafu {
                ordinal: header.format.ordinal
            }
        );

        Ok(header)
    }

    /// Returns `Ok(())` iff `header` is `None`.
    #[inline(always)]
    fn ensure_no_header(header: Option<DataHeader>) -> Result<(), CodecError> {
        ensure!(
            header.is_none(),
            UnexpectedDataFormatSnafu {
                expected: Self::FORMAT,
                actual: header,
            }
        );

        Ok(())
    }
}

/// A thing that [`Reads`] [`Decodable`] data.
///
/// This trait is automatically implemented for
/// any type that [`Reads`].
pub trait ReadsDecodable: Reads {
    /// Reads and decodes a sequence of data into
    /// a new, default instance of `T`.
    ///
    /// This function will attempt to read a [`DataHeader`]
    /// if the `data`'s [`Format::is_structured`](crate::codec::Format::is_structured).
    fn read_data<T: Decodable + Default>(&mut self) -> Result<T, CodecError> {
        let mut default = T::default();
        self.read_data_into(&mut default)?;
        Ok(default)
    }

    /// Reads and decodes a sequence of data into `data`.
    ///
    /// This function will attempt to read a [`DataHeader`]
    /// if the `data`'s [`Format::is_structured`](crate::codec::Format::is_structured).
    fn read_data_into<T: Decodable>(&mut self, data: &mut T) -> Result<(), CodecError> {
        if T::FORMAT.is_structured() {
            let header = self.read_data()?;
            data.decode(self, Some(header))?;
        } else {
            data.decode(self, None)?;
        }

        Ok(())
    }

    /// Skips to the end of the next `length` bytes of data.
    fn skip_blob(&mut self, length: usize) -> Result<(), CodecError> {
        let mut skipped = 0;
        let mut buf = [0; TEMP_BUFFER_SIZE];
        while skipped < length {
            let remaining = length - skipped;
            if remaining < TEMP_BUFFER_SIZE {
                skipped += self.read(&mut buf[..remaining])?;
            } else {
                skipped += self.read(&mut buf)?;
            }
        }
        Ok(())
    }

    /// Skips to the end of the next encoded sequence of data,
    /// returning the total number of bytes skipped.
    fn skip_data(&mut self) -> Result<usize, CodecError> {
        let mut read = 0;

        // Decode data header.
        let header: DataHeader = self.read_data()?;
        read += DataHeader::FORMAT.as_data_format().blob_size as usize;
        let data_format = header.format;

        // Decode all data in the sequence, skipping
        // their blobs and recursively skipping data fields.
        for _ in 0..header.count {
            read += self.skip_data_with_format(data_format)?;
        }

        Ok(read)
    }

    /// Skips to the end of the next encoded instance
    /// of data with `format`, returning the total number
    /// of bytes skipped.
    fn skip_data_with_format(&mut self, format: DataFormat) -> Result<usize, CodecError> {
        let mut read = 0;

        // Skip the blob.
        self.skip_blob(format.blob_size as usize)?;
        read += format.blob_size as usize;

        // Skip all data fields recursively.
        for _ in 0..format.data_fields {
            read += self.skip_data()?;
        }

        Ok(read)
    }
}

impl<T: Reads + ?Sized> ReadsDecodable for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codec::tests::*, types::Text};

    #[test]
    fn decodes() -> Result<(), CodecError> {
        // Encode test data.
        let mut bytes = Vec::new();
        encode_test_data(&mut bytes);
        let mut bytes = bytes.as_slice();

        // Decode the header.
        let header: DataHeader = bytes.read_data()?;
        assert_eq!(1, header.count);
        assert_eq!(0, header.format.ordinal);
        assert_eq!(12, header.format.blob_size);
        assert_eq!(1, header.format.data_fields);

        // Decode blob fields.
        assert_eq!(TestData::default().num_a, bytes.read_data()?);
        assert_eq!(TestData::default().num_b, bytes.read_data()?);

        // Decode text.
        let mut text = Text::default();
        bytes.read_data_into(&mut text)?;
        assert_eq!(TestData::default().text, text);

        Ok(())
    }

    #[test]
    fn splits_off_group_sequences() -> Result<(), CodecError> {
        // Pre encode a sequence of expected data.
        let mut expected = vec![];
        encode_test_data(&mut expected);

        // Encode two group sequences into `bytes`.
        let mut bytes = vec![];
        encode_test_data(&mut bytes);
        encode_test_data(&mut bytes);

        // Keep a slice of the full bytes, in
        // addition to the slice we'll traverse
        // during decoding.
        let original_bytes = bytes.as_slice();
        let mut bytes = bytes.as_slice();

        // Split off and decode the first sequence.
        let data_one_length = bytes.skip_data()?;
        let (data_one, original_bytes) = original_bytes.split_at(data_one_length);
        assert_eq!(expected, data_one);

        // Split off and decode the second sequence.
        let data_two_length = bytes.skip_data()?;
        let (data_two, _) = original_bytes.split_at(data_two_length);
        assert_eq!(expected, data_two);

        Ok(())
    }
}
