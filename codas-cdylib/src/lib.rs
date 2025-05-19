// Use the README file as the root-level
// docs for this library.
#![doc = include_str!("../README.md")]

#[cfg(all(feature = "wasm", feature = "python"))]
compile_error!("features `wasm` and `python` are mutually exclusive");

use ::codas::{
    parse::ParseError,
    stream::StreamError,
    types::{
        binary::{self, hex_from_bytes, BinaryError},
        cryptography::{
            CryptoError, CryptoKeys, CryptoSigns, EncryptedData, HasCryptoPublicKey,
            PrivateKeyBytes,
        },
    },
};

#[cfg(feature = "python")]
#[pyo3::prelude::pymodule]
fn codas(m: &pyo3::prelude::Bound<'_, pyo3::prelude::PyModule>) -> pyo3::prelude::PyResult<()> {
    use pyo3::prelude::*;

    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(codegen, m)?)?;
    Ok(())
}

/// Parses `markdown` into a Coda.
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "python", pyo3::prelude::pyfunction)]
pub fn parse(markdown: &str) -> Result<Coda, Error> {
    Ok(Coda {
        coda: ::codas::parse::parse(markdown)?,
    })
}

/// Generates API bindings for `coda` in `language`.]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "python", pyo3::prelude::pyfunction)]
pub fn codegen(coda: &Coda, language: &str) -> Result<String, Error> {
    let mut codegen = vec![];

    match language.trim().to_lowercase().as_str() {
        "open-api" => ::codas::langs::open_api::generate_spec(&coda.coda, &mut codegen),
        "python" => ::codas::langs::python::generate_types(&coda.coda, &mut codegen),
        "rust" => ::codas::langs::rust::generate_types(&coda.coda, &mut codegen, true),
        "typescript" => ::codas::langs::typescript::generate_types(&coda.coda, &mut codegen),
        language => {
            return Err(Error::Internal(format!(
                "unsuppored coda codegen language: {language}"
            )))
        }
    }?;

    Ok(String::from_utf8_lossy(&codegen).to_string())
}

/// ## Unstable
///
/// Encrypts `string` with `key`, returning
/// the encrypted data formatted via [EncryptedData::to_hex].
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "python", pyo3::prelude::pyfunction)]
pub fn encrypt_str(key: &str, string: &str) -> Result<String, Error> {
    Ok(EncryptedData::new(key.as_bytes(), string.as_bytes())?.to_hex())
}

/// ## Unstable
///
/// Decrypts `string` containing hexadecimal-encoded bytes
/// with `key`, returning the decrypted data as a hexadecimal string.
///
/// `string` must be in the same format expected by
/// [`EncryptedData::from_hex`].
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "python", pyo3::prelude::pyfunction)]
pub fn decrypt_hex(key: &str, string: &str) -> Result<String, Error> {
    // TODO: For legacy compatibility (?),
    //       replace ':' in strings with '-'.
    let string = string.replace(':', "-");

    // Run decryption.
    let encrypted = EncryptedData::from_hex(&string)?;
    let decrypted = encrypted.decrypt(key.as_bytes())?;

    Ok(hex_from_bytes(&decrypted).to_string())
}

/// ## Unstable
///
/// Extracts the public key from a HEX-encoded private key,
/// returning a HEX-encoded public key.
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "python", pyo3::prelude::pyfunction)]
pub fn extract_public_key(private_key: &str) -> Result<String, Error> {
    let mut private_key_bytes = PrivateKeyBytes::NULL;
    private_key_bytes.from_hex(private_key)?;
    let private_key = CryptoKeys::from_private(private_key_bytes)?;

    let public_key = private_key.public_key_bytes();

    Ok(binary::hex_from_bytes(&public_key).to_string())
}

/// ## Unstable
///
/// Returns a HEX-encoded signature from `private_key` for `message`.
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "python", pyo3::prelude::pyfunction)]
pub fn sign(private_key: &str, message: &str) -> Result<String, Error> {
    let mut private_key_bytes = PrivateKeyBytes::NULL;
    private_key_bytes.from_hex(private_key)?;
    let private_key = CryptoKeys::from_private(private_key_bytes)?;

    Ok(binary::hex_from_bytes(&private_key.sign(&[message.as_bytes()])?).to_string())
}

/// Exported representation of a [`::codas::types::Coda`].
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
pub struct Coda {
    coda: ::codas::types::Coda,
}

#[derive(Debug)]
pub enum Error {
    Internal(String),
}

impl From<ParseError> for Error {
    fn from(value: ParseError) -> Self {
        Self::Internal(value.to_string())
    }
}

impl From<StreamError> for Error {
    fn from(value: StreamError) -> Self {
        Self::Internal(value.to_string())
    }
}

impl From<BinaryError> for Error {
    fn from(value: BinaryError) -> Self {
        Self::Internal(value.to_string())
    }
}

impl From<CryptoError> for Error {
    fn from(value: CryptoError) -> Self {
        Self::Internal(value.to_string())
    }
}

#[cfg(feature = "wasm")]
impl From<Error> for wasm_bindgen::JsValue {
    fn from(value: Error) -> Self {
        match value {
            Error::Internal(value) => value.into(),
        }
    }
}

#[cfg(feature = "python")]
impl From<Error> for pyo3::PyErr {
    fn from(value: Error) -> Self {
        match value {
            Error::Internal(value) => pyo3::exceptions::PyValueError::new_err(value),
        }
    }
}

#[cfg(test)]
mod test {
    use codas::types::binary::bytes_from_hex;

    use crate::{decrypt_hex, encrypt_str};

    #[test]
    pub fn encryption() {
        let encrypted = encrypt_str("key", "message").unwrap();
        let decrypted = decrypt_hex("key", &encrypted).unwrap();
        assert_eq!(
            "message",
            String::from_utf8_lossy(&bytes_from_hex(&decrypted).unwrap())
        );
    }
}
