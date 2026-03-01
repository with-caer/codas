//! Numeric data types (including `bool`).

use crate::codec::{
    CodecError, DataHeader, Decodable, Encodable, Format, ReadsDecodable, WritesEncodable,
};

/// Implements codec traits for a native numeric type.
macro_rules! numeric_impls {
    (
        /// Primitive type to generate codec for.
        $primitive_type:ident,

        /// Expression which evaluates to the size
        /// of the primitive, in bytes.
        $primitive_size:expr
    ) => {
        impl $crate::codec::Encodable for $primitive_type {
            #[doc = concat!(
                                                "Encoded as a [`Format::Blob(",
                                                stringify!($primitive_size), ")`](Format::Blob) ",
                                                "containing the result of [`",
                                                stringify!($primitive_type), "::to_le_bytes`]."
                                            )]
            const FORMAT: $crate::codec::Format = $crate::codec::Format::Blob($primitive_size);

            fn encode(
                &self,
                writer: &mut (impl $crate::codec::WritesEncodable + ?Sized),
            ) -> Result<(), $crate::codec::CodecError> {
                writer.write_all(&self.to_le_bytes())?;
                Ok(())
            }
        }

        impl $crate::codec::Decodable for $primitive_type {
            fn decode(
                &mut self,
                reader: &mut impl $crate::codec::ReadsDecodable,
                header: Option<$crate::codec::DataHeader>,
            ) -> Result<(), $crate::codec::CodecError> {
                Self::ensure_no_header(header)?;
                let mut bytes = [0u8; $primitive_size];
                reader.read_exact(&mut bytes)?;
                *self = $primitive_type::from_le_bytes(bytes);
                Ok(())
            }
        }
    };
}

numeric_impls!(u8, 1);
numeric_impls!(u16, 2);
numeric_impls!(u32, 4);
numeric_impls!(u64, 8);
numeric_impls!(i8, 1);
numeric_impls!(i16, 2);
numeric_impls!(i32, 4);
numeric_impls!(i64, 8);
numeric_impls!(f32, 4);
numeric_impls!(f64, 8);

impl Encodable for bool {
    /// Encoded as a [`u8`], with a value of
    /// `1` for `true` and `0` for `false`.
    const FORMAT: Format = u8::FORMAT;

    fn encode(&self, writer: &mut (impl WritesEncodable + ?Sized)) -> Result<(), CodecError> {
        if *self {
            writer.write_data(&1u8)
        } else {
            writer.write_data(&0u8)
        }
    }
}

impl Decodable for bool {
    fn decode(
        &mut self,
        reader: &mut impl ReadsDecodable,
        header: Option<DataHeader>,
    ) -> Result<(), CodecError> {
        Self::ensure_no_header(header)?;
        *self = reader.read_data::<u8>()? == 1;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use core::{f32, f64};

    use crate::codec::{ReadsDecodable, WritesEncodable};

    #[test]
    fn test_u8_codec() {
        let value = 255u8;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_u16_codec() {
        let value = 65535u16;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_u32_codec() {
        let value = 4294967295u32;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_u64_codec() {
        let value = 18446744073709551615u64;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_i8_codec() {
        let value = -128i8;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_i16_codec() {
        let value = -32768i16;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_i32_codec() {
        let value = -2147483648i32;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_i64_codec() {
        let value = -9223372036854775808i64;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_f32_codec() {
        let value = f32::consts::PI;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_f64_codec() {
        let value = f64::consts::E;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_bool_codec() {
        let value = true;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);

        let value = false;
        let mut encoded = vec![];
        encoded.write_data(&value).expect("encoded");
        let decoded = encoded.as_slice().read_data().expect("decoded");
        assert_eq!(value, decoded);
    }
}
