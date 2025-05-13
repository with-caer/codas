// Use the README file as the root-level
// docs for this library.
#![doc = include_str!("../README.md")]

use codas::types::cryptography::EncryptedData;
use wasm_bindgen::prelude::*;

/// Encrypts `string` with `key`, returning
/// the encrypted data formatted via [EncryptedData::to_hex].
#[wasm_bindgen]
pub fn encrypt_str(key: &str, string: &str) -> Result<String, String> {
    Ok(EncryptedData::new(key.as_bytes(), string.as_bytes())
        .map_err(|e| e.to_string())?
        .to_hex())
}

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
