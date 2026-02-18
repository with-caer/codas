use std::io::Read;

use codas::{
    codec::TEMP_BUFFER_SIZE,
    types::cryptography::{
        CryptoHasher, CryptoKeys, CryptoSigns, EncryptedData, HasCryptoPublicKey, PrivateKeyBytes,
    },
};

use super::{open_file_or_stdin, CryptographyCommand};

/// Executes `command` locally.
pub fn execute_cryptography_command(command: CryptographyCommand) {
    match command {
        CryptographyCommand::Hash { source } => {
            // Open input source.
            let mut bytes = open_file_or_stdin(source).expect("source doesn't exist");

            // Hash all bytes.
            let mut buffer = Vec::with_capacity(TEMP_BUFFER_SIZE);
            let mut hasher = CryptoHasher::default();
            bytes.read_to_end(&mut buffer).expect("source read failed");
            hasher.write(&buffer);
            let hash = hasher.finalize();

            // Display the HEX-encoded hash.
            eprintln!("Blake3 Hash (HEX): {}", hash.to_hex());
        }
        CryptographyCommand::Keygen { passphrase } => {
            // Generate a keypair.
            let keys = CryptoKeys::generate();

            // Display the public key.
            eprintln!(
                "Ed25519 Public Key (HEX): {}",
                keys.public_key_bytes().to_hex()
            );

            // Encrypt the private key.
            let encrypted_keys = EncryptedData::new(passphrase.as_bytes(), &keys.into_private())
                .expect("encryption failed");

            // Display the encrypted data.
            eprintln!(
                "Encrypted Ed25519 Private Key (HEX): {}",
                encrypted_keys.to_hex()
            );
        }
        CryptographyCommand::Sign {
            keypair,
            passphrase,
            source,
        } => {
            // Open keypair.
            let mut keys = std::fs::File::open(keypair).expect("keypair data doesn't exist");
            let mut keys_string = String::new();
            keys.read_to_string(&mut keys_string)
                .expect("keypair data is empty");
            let keys_string = keys_string.trim();

            // Decrypt keypair.
            let encrypted_keys =
                EncryptedData::from_hex(keys_string).expect("encrypted keypair is malformed");
            let decrypted_keys = encrypted_keys
                .decrypt(passphrase.as_bytes())
                .expect("invalid keypair passphrase");
            let decrypted_keys: PrivateKeyBytes =
                PrivateKeyBytes::try_from(decrypted_keys.as_slice())
                    .expect("decrypted keypair is malformed");
            let keys =
                CryptoKeys::from_private(decrypted_keys).expect("decrypted keypair is invalid");

            // Display the decrypted public key.
            eprintln!(
                "Decrypted Ed25519 Public Key (HEX): {}",
                keys.public_key_bytes().to_hex()
            );

            // Open input source.
            let mut bytes = open_file_or_stdin(source).expect("source doesn't exist");

            // Sign all bytes.
            let mut buffer = Vec::with_capacity(TEMP_BUFFER_SIZE);
            bytes.read_to_end(&mut buffer).expect("source read failed");
            let signature = keys.sign(&[&buffer]).expect("signing failed");

            // Display the HEX-encoded hash.
            eprintln!("ED25519 Signature (HEX): {}", signature.to_hex());
        }
    }
}
