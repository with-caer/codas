//! Binary data encoder and decoder ("codec").
//!
//! This codec is meant to be:
//!
//! 1. _Accessible_, so that it doesn't require
//!    specialized knowledge (beyond foundational
//!    coding skills) to implement on any platform.
//! 2. _Streamable_, so that it can encode into
//!    and decode from binary data streams which
//!    only support sequential reads and writes
//!    (i.e., no "backtracking" or allocations).
//! 3. _Upgradeable_, so that the format of encoded
//!    data can evolve without breaking systems
//!    that rely on outdated decoders.
//!
//! Aspects of this codec were inspired by
//! [Simple Binary Encoding](https://github.com/real-logic/simple-binary-encoding)
//! and [Cap'n Proto](https://capnproto.org/).
//!
//! ## Bits, Bytes, Endians, and Alignments
//!
//! Despite the widespread use of the word **byte**,
//! there _isn't_ a universal standard for what
//! a byte _is_.
//!
//! For the sake of clarity, in our documentation,
//! **a byte is `8` bits of data**. Some people call
//! this precise definition an **octet**; we prefer
//! "byte" because "octet" is a bit niche.
//!
//! ### Endianness
//!
//! When bytes are transmitted between systems,
//! those systems might not process the bytes
//! in the same exact order.
//!
//! This difference is due to **endianness**,
//! which is the order a computer reads the
//! bytes representing a number. For example,
//! consider the following three bytes:
//!
//! ```binary
//! 01101000 01101001 00100001
//! ```
//!
//! A "little-endian" computer will read these
//! bytes left-to-right, interpreting them as
//! the number `6,842,657`.
//!
//! A "big-endian" computer will read these
//! bytes right-to-left, decoding them as the
//! number `1,480,324`.
//!
//! This codec encodes numbers in **little-endian** format.
//!
//! ### Alignment
//!
//! When computers are asked to read a byte,
//! they usually _don't_ read a single byte;
//! they read a _batch_ of bytes called a **word**.
//! Words' sizes vary between computers, but
//! they're usually `4` or `8` bytes long.
//!
//! Because computers process bytes in words,
//! we can improve the performance of our code
//! by **aligning** our data to be a size
//! that is evenly divisible by a word.
//!
//! > _Note_: The most common alignment strategy
//! > is to re-order the components of a data
//! > structure from largest to smallest,
//! > inserting padding after each component so
//! > that the data structure (up to the end of
//! > the padding) has a size that is evenly
//! > divisible by a word.
//!
//! Data encoded by this codec is **unaligned**, with
//! no padding bytes within or around data. By not aligning
//! data, the codec sacrifices _some_ performance in
//! exchange for a smaller encoded size, and a
//! vasly simplified codec.
//!
//! However, this codec _does_ accomodate aligned data:
//! All encoded metadata is aligned to an
//! **`8` byte word boundary**., meaning every encoded
//! data is guaranteed to start on an `8`-byte boundary so
//! long as the blob section of any [`Format::Data`]
//! is `8`-byte aligned.
//!
//! ## The Encoding
//!
//! This codec encodes data as a structured sequence of
//! bytes containing, in order:
//!
//! 1. A [`DataHeader`] describing the format of
//!    the encoded data sequence, and the number of
//!    data encoded in the sequence.
//! 2. For each encoded data following the header:
//!    1. The data's _blob_ fields, encoded in some
//!       predetermined documented order.
//!    2. The data's _data_ fields, each preceded by their
//!       own [`DataHeader`], and encoded in some
//!       predetermined documented order.
//!
//! Each [`DataHeader`] contains:
//!
//! Type | Description
//! -----|-----------
//! `u16`| The number of data following the header; `0` for no data, `1` for one data, and so on.
//! `u16`| The ordinal of the data's type in it's documentation, defaulting to `0` ("unspecified").
//! `u16`| The total size in bytes of the data's [`Format::Blob`] fields, defaulting to `0` (none).
//! `u16`| The total number of the data's [`Format::Data`] fields, defaulting to `0` (none).
//!
//! Because each [`DataHeader`] contains a count
//! of how many distinct sequences of data follow
//! the header, the encoding is identical for an
//! empty sequence of data, a single sequence of
//! data, and a list of sequences of data.
//!
//! Data is not encoded with any additional metadata
//! (e.g., field or type names). The [`DataHeader`]
//! provides enough information to _traverse_ any data,
//! but the data's contents won't be useful without
//! having the data's corresponding documentation.
use snafu::{Backtrace, Snafu};

use crate::stream::StreamError;

// Expose encoder and decoder APIs as part of this module,
// while keeping them in separate files to reduce clutter.
mod decode;
mod encode;
pub use decode::*;
pub use encode::*;

/// Numeric type used for describing a [`Format`].
pub type FormatMetadata = u16;

/// The low-level encoding format of some data.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Default)]
pub enum Format {
    /// Unstructured sequence of binary
    /// data with a fixed size in bytes.
    Blob(FormatMetadata),

    /// Structured sequence of data containing
    /// [`Format::Blob`]s and/or other [`Format::Data`].
    Data(DataFormat),

    /// [`Format::Data`] with an unspecified format.
    ///
    /// Data with this format may encode to and
    /// from several kinds of [`Format::Data`].
    #[default]
    Fluid,
}

impl Format {
    /// Shorthand to return a new empty
    /// [`Format::Data`] with `ordinal`.
    pub const fn data(ordinal: FormatMetadata) -> Self {
        Self::Data(DataFormat {
            ordinal,
            blob_size: 0,
            data_fields: 0,
        })
    }

    /// Returns true iff `self` is a structured
    /// data format (i.e., [`Format::Data`] or [`Format::Fluid`]).
    pub const fn is_structured(self) -> bool {
        matches!(self, Self::Data(..) | Self::Fluid)
    }

    /// Returns a new `self` containing additional
    /// data with `other`'s format.
    ///
    /// This operation is _not_ commutative; that
    /// is, `self.with(other)` and `other.with(self)`
    /// may return different formats.
    pub const fn with(self, other: Self) -> Self {
        match (self, other) {
            // Adding blobs together yields a bigger blob.
            (Format::Blob(f1), Format::Blob(f2)) => Self::Blob(f1 + f2),

            // Adding data to a blob yields data containing
            // the blob and a single data field.
            (Format::Blob(size), Format::Data(_)) | (Format::Blob(size), Format::Fluid) => {
                DataFormat {
                    ordinal: 0,
                    blob_size: size,
                    data_fields: 1,
                }
                .as_format()
            }

            // Adding blobs to data yields the same data,
            // with a bigger blob.
            (Format::Data(format), Format::Blob(size)) => DataFormat {
                ordinal: format.ordinal,
                blob_size: format.blob_size + size,
                data_fields: format.data_fields,
            }
            .as_format(),

            // Adding data to data yields the same data,
            // with more data fields.
            (Format::Data(format), Format::Data(_)) | (Format::Data(format), Format::Fluid) => {
                DataFormat {
                    ordinal: format.ordinal,
                    blob_size: format.blob_size,
                    data_fields: format.data_fields + 1,
                }
                .as_format()
            }

            // Adding anything to a fluid format does nothing.
            (Format::Fluid, Format::Blob(_))
            | (Format::Fluid, Format::Data(_))
            | (Format::Fluid, Format::Fluid) => Format::Fluid,
        }
    }

    /// Returns a [`DataFormat`] equivalent to this format.
    pub const fn as_data_format(self) -> DataFormat {
        match self {
            // Blobs are returned as unspecified data
            // containing the blob.
            Format::Blob(size) => DataFormat {
                ordinal: 0,
                blob_size: size,
                data_fields: 0,
            },

            // Data are returned as-is.
            Format::Data(format) => format,

            // Fluids are returned as unspecified data
            // containing a single, unspecified data field.
            Format::Fluid => DataFormat {
                ordinal: 0,
                blob_size: 0,
                data_fields: 1,
            },
        }
    }

    /// Encodes this format's default value to `writer`.
    pub fn encode_default_value(
        &self,
        writer: &mut (impl encode::WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        match self {
            Format::Blob(size) => {
                for _ in 0..*size {
                    0u8.encode(writer)?;
                }

                Ok(())
            }

            Format::Data(..) | Format::Fluid => Ok(()),
        }
    }

    /// Encodes this format's default header to `writer`.
    pub fn encode_default_header(
        &self,
        writer: &mut (impl encode::WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        match self {
            Format::Blob(..) => Ok(()),

            Format::Data(format) => DataHeader {
                count: 0,
                format: *format,
            }
            .encode(writer),

            Format::Fluid => DataHeader {
                count: 0,
                format: DataFormat {
                    ordinal: 0,
                    blob_size: 0,
                    data_fields: 0,
                },
            }
            .encode(writer),
        }
    }
}

impl Encodable for Format {
    const FORMAT: Format = Format::Fluid;

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        match self {
            Format::Blob(size) => writer.write_data(size),
            Format::Data(format) => {
                writer.write_data(&format.ordinal)?;
                writer.write_data(&format.blob_size)?;
                writer.write_data(&format.data_fields)
            }
            Format::Fluid => Ok(()),
        }
    }

    fn encode_header(
        &self,
        writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        let header = match self {
            Format::Blob(_) => DataHeader {
                count: 1,
                format: DataFormat {
                    ordinal: 1,
                    blob_size: 2,
                    data_fields: 0,
                },
            },
            Format::Data(_) => DataHeader {
                count: 1,
                format: DataFormat {
                    ordinal: 2,
                    blob_size: 2 + 2 + 2,
                    data_fields: 0,
                },
            },
            Format::Fluid => DataHeader {
                count: 1,
                format: DataFormat {
                    ordinal: 3,
                    blob_size: 0,
                    data_fields: 0,
                },
            },
        };

        header.encode(writer)
    }
}

impl Decodable for Format {
    fn decode(
        &mut self,
        reader: &mut (impl ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        let header = Self::ensure_header(header, &[1, 2, 3])?;

        match header.format.ordinal {
            1 => {
                let mut size = 0;
                reader.read_data_into(&mut size)?;
                *self = Format::Blob(size);
            }

            2 => {
                let mut ordinal = 0;
                reader.read_data_into(&mut ordinal)?;
                let mut blob_size = 0;
                reader.read_data_into(&mut blob_size)?;
                let mut data_fields = 0;
                reader.read_data_into(&mut data_fields)?;
                *self = Format::Data(DataFormat {
                    ordinal,
                    blob_size,
                    data_fields,
                });
            }

            3 => {
                *self = Format::Fluid;
            }

            _ => unreachable!(),
        }

        Ok(())
    }
}

/// Contents of a [`Format::Data`].
#[derive(Default, Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct DataFormat {
    /// Ordinal identifier of the data's
    /// type in it's corresponding documentation,
    /// or `0` if the type is unspecified.
    pub ordinal: FormatMetadata,

    /// The total size in bytes of the
    /// [`Format::Blob`] fields in the data.
    pub blob_size: FormatMetadata,

    /// The total number of [`Format::Data`]
    /// fields in the data.
    pub data_fields: FormatMetadata,
}

impl DataFormat {
    /// Returns a [`Format`] equivalent to
    /// this data format.
    pub const fn as_format(self) -> Format {
        Format::Data(self)
    }
}

/// Header preceding a sequence of zero or more
/// data encoded with the same [`DataFormat`].
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct DataHeader {
    /// The number of encoded data following this header,
    /// each having the same format as [`Self::format`].
    pub count: FormatMetadata,

    /// The format of the data following this header.
    pub format: DataFormat,
}

impl Encodable for DataHeader {
    /// Encoded as a [`Format::Blob(8)`](Format::Blob)
    /// containing, in order:
    ///
    /// 1. [`Self::count`]
    /// 2. [`DataFormat::ordinal`]
    /// 3. [`DataFormat::blob_size`]
    /// 4. [`DataFormat::data_fields`]
    ///
    /// All values are encoded as [`u16`].
    const FORMAT: Format = Format::Blob(0)
        .with(Format::Blob((u16::BITS / 8) as FormatMetadata))
        .with(Format::Blob((u16::BITS / 8) as FormatMetadata))
        .with(Format::Blob((u16::BITS / 8) as FormatMetadata))
        .with(Format::Blob((u16::BITS / 8) as FormatMetadata));

    #[inline(always)]
    fn encode(
        &self,
        writer: &mut (impl encode::WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        writer.write_all(&self.count.to_le_bytes())?;
        writer.write_all(&self.format.ordinal.to_le_bytes())?;
        writer.write_all(&self.format.blob_size.to_le_bytes())?;
        writer.write_all(&self.format.data_fields.to_le_bytes())?;
        Ok(())
    }

    /// Headers have no header, since
    /// they _are_ the header; this function
    /// is a no-op.
    #[inline(always)]
    fn encode_header(
        &self,
        _writer: &mut (impl WritesEncodable + ?Sized),
    ) -> Result<(), CodecError> {
        Ok(())
    }
}

impl Decodable for DataHeader {
    fn decode(
        &mut self,
        reader: &mut (impl decode::ReadsDecodable + ?Sized),
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        Self::ensure_no_header(header)?;

        // Temporary buffer for the decoded bytes.
        let mut bytes = [0u8; (u16::BITS / 8) as usize];

        reader.read_exact(&mut bytes)?;
        self.count = u16::from_le_bytes(bytes);
        reader.read_exact(&mut bytes)?;
        self.format.ordinal = u16::from_le_bytes(bytes);
        reader.read_exact(&mut bytes)?;
        self.format.blob_size = u16::from_le_bytes(bytes);
        reader.read_exact(&mut bytes)?;
        self.format.data_fields = u16::from_le_bytes(bytes);

        Ok(())
    }
}

/// Enumeration of errors that may occur while
/// encoding or decoding data.
#[derive(Debug, Snafu)]
pub enum CodecError {
    /// An encoder was asked to encode a blob
    /// as structured data.
    #[snafu(display("can't encode data with the format {format:?} as structured data"))]
    UnstructuredFormat {
        format: Format,
        backtrace: Backtrace,
    },

    /// A header for the wrong data format was given to a decoder.
    #[snafu(display("expected to decode {expected:?}, but found {actual:?}"))]
    UnexpectedDataFormat {
        expected: Format,
        actual: Option<DataHeader>,
        backtrace: Backtrace,
    },

    /// An unsupported data format ordinal was given to a decoder.
    #[snafu(display("unsupported data format (ordinal {ordinal:?})"))]
    UnsupportedDataFormat {
        ordinal: FormatMetadata,
        backtrace: Backtrace,
    },

    /// A decoder expected to decode more blob fields' data.
    #[snafu(display("expected to decode {length} more bytes of blob field data"))]
    MissingBlobLength { length: FormatMetadata },

    /// A decoder expected to decode more data fields.
    #[snafu(display("expected to decode {count} more fields of data"))]
    MissingDataFields { count: FormatMetadata },

    /// An error occurred while reading or
    /// writing the underlying data stream.
    #[snafu(display("error when reading or writing from a data stream: {source}"))]
    Stream { source: StreamError },
}

impl From<StreamError> for CodecError {
    fn from(value: StreamError) -> Self {
        Self::Stream { source: value }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::{stream::Writes, types::Text};

    /// Test data for codecs.
    pub(super) struct TestData {
        pub num_a: i32,
        pub num_b: u64,
        pub text: Text,
    }

    impl Encodable for TestData {
        const FORMAT: Format = Format::data(0)
            // 32-bit signed integer
            .with(i32::FORMAT)
            // 64-bit unsigned integer
            .with(u64::FORMAT)
            // Text
            .with(Text::FORMAT);

        fn encode(
            &self,
            writer: &mut (impl encode::WritesEncodable + ?Sized),
        ) -> Result<(), CodecError> {
            writer.write_data(&self.num_a)?;
            writer.write_data(&self.num_b)?;
            writer.write_data(&self.text)?;
            Ok(())
        }
    }

    impl Default for TestData {
        fn default() -> Self {
            Self {
                num_a: -3i32,
                num_b: 333u64,
                text: "var-length field!".into(),
            }
        }
    }

    /// _Manually_ encodes a single [`TestData`]
    /// into `bytes`.
    pub(super) fn encode_test_data(bytes: &mut Vec<u8>) {
        // Encode header.
        bytes.write_all(&1u16.to_le_bytes()).unwrap(); // count
        bytes.write_all(&0u16.to_le_bytes()).unwrap(); // ordinal
        bytes.write_all(&12u16.to_le_bytes()).unwrap(); // blob size
        bytes.write_all(&1u16.to_le_bytes()).unwrap(); // data fields

        // Encode blob fields.
        bytes
            .write_all(&TestData::default().num_a.to_le_bytes())
            .unwrap();
        bytes
            .write_all(&TestData::default().num_b.to_le_bytes())
            .unwrap();

        // Encode data
        bytes
            .write_all(&(TestData::default().text.len() as u16).to_le_bytes())
            .unwrap(); // count
        bytes.write_all(&0u16.to_le_bytes()).unwrap(); // ordinal
        bytes.write_all(&1u16.to_le_bytes()).unwrap(); // blob size
        bytes.write_all(&0u16.to_le_bytes()).unwrap(); // data fields
        bytes
            .write_all(TestData::default().text.as_bytes())
            .unwrap();
    }

    /// Test codec for [`Format`]s.
    #[test]
    fn format_codec() {
        // Blobs.
        let blob_format = Format::Blob(69);
        let mut bytes = vec![];
        bytes.write_data(&blob_format).unwrap();
        assert_eq!(blob_format, bytes.as_slice().read_data().unwrap());

        // Data.
        let data_format = Format::Data(DataFormat {
            ordinal: 1337,
            blob_size: 9001,
            data_fields: 42,
        });
        let mut bytes = vec![];
        bytes.write_data(&data_format).unwrap();
        assert_eq!(data_format, bytes.as_slice().read_data().unwrap());

        // Fluids.
        let fluid_format = Format::Fluid;
        let mut bytes = vec![];
        bytes.write_data(&fluid_format).unwrap();
        assert_eq!(fluid_format, bytes.as_slice().read_data().unwrap());
    }
}
