// Use the README file as the root-level
// docs for this library.
#![doc = include_str!("../README.md")]

use codas::types::cryptography::EncryptedData;
use wasm_bindgen::prelude::*;

/// Parses `markdown` into a Coda.
#[wasm_bindgen]
pub fn parse(markdown: &str) -> Result<Coda, String> {
    match codas::parse::parse(markdown) {
        Ok(coda) => Ok(Coda { coda }),
        Err(e) => Err(e.to_string()),
    }
}

/// Generates API bindings for `coda` in `language`.]
#[wasm_bindgen]
pub fn codegen(coda: &Coda, language: &str) -> Result<String, String> {
    let mut codegen = vec![];

    match language.trim().to_lowercase().as_str() {
        "open-api" => codas::langs::open_api::generate_spec(&coda.coda, &mut codegen),
        "python" => codas::langs::python::generate_types(&coda.coda, &mut codegen),
        "rust" => codas::langs::rust::generate_types(&coda.coda, &mut codegen, true),
        "typescript" => codas::langs::typescript::generate_types(&coda.coda, &mut codegen),
        language => return Err(format!("unsuppored coda codegen language: {language}")),
    }
    .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&codegen).to_string())
}

/// ## Unstable
///
/// Encrypts `string` with `key`, returning
/// the encrypted data formatted via [EncryptedData::to_hex].
#[wasm_bindgen]
pub fn encrypt_str(key: &str, string: &str) -> Result<String, String> {
    Ok(EncryptedData::new(key.as_bytes(), string.as_bytes())
        .map_err(|e| e.to_string())?
        .to_hex())
}

/// ## Unstable
///
/// Decrypts `string` with `key`, returning the
/// decrypted `string`.
///
/// `string` must be in the same format expected by
/// [`EncryptedData::from_hex`].
#[wasm_bindgen]
pub fn decrypt_str(key: &str, string: &str) -> Result<String, String> {
    // TODO: For legacy compatibility (?),
    //       replace ':' in strings with '-'.
    let string = string.replace(':', "-");

    // Run decryption.
    let encrypted = EncryptedData::from_hex(&string).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(
        encrypted
            .decrypt(key.as_bytes())
            .map_err(|e| e.to_string())?
            .as_slice(),
    )
    .to_string())
}

/// Exported representation of a [`codas::types::Coda`].
#[wasm_bindgen]
pub struct Coda {
    coda: codas::types::Coda,
}

#[cfg(test)]
mod test {
    use crate::{decrypt_str, encrypt_str};

    #[test]
    pub fn encryption() {
        let encrypted = encrypt_str("key", "message").unwrap();
        let decrypted = decrypt_str("key", &encrypted).unwrap();
        assert_eq!("message", decrypted);
    }
}
