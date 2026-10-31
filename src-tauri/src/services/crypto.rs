use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Encryption failure")]
    Encryption,
    #[error("Decryption failure (invalid key, ciphertext, or authentication tag)")]
    Decryption,
    #[error("Invalid key length: expected 32 bytes, got {0}")]
    InvalidKeyLength(usize),
    #[error("Ciphertext too short: must be at least 28 bytes (12 nonce + 16 tag)")]
    CiphertextTooShort,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub app_name: String,
    pub encryption_key: String, // Base64 encoded 32-byte key
    pub created_at: String,
}

/// Resolves the application data directory:
/// - Linux: ~/.local/share/olahpos/
/// - Windows: %APPDATA%/olahpos/
pub fn get_app_data_dir() -> Result<PathBuf, std::io::Error> {
    let base_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("./data"));
    let app_dir = base_dir.join("olahpos");
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)?;
    }
    Ok(app_dir)
}

/// Loads or generates the 32-byte symmetric encryption key from config.json
pub fn get_or_create_app_key() -> Result<[u8; 32], CryptoError> {
    let app_dir = get_app_data_dir()?;
    let config_path = app_dir.join("config.json");

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let key_bytes = BASE64.decode(&config.encryption_key)?;
        if key_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyLength(key_bytes.len()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(key)
    } else {
        // Generate new random 32-byte key using CSPRNG
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        let config = AppConfig {
            app_name: "OlahPOS".to_string(),
            encryption_key: BASE64.encode(key),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let json_content = serde_json::to_string_pretty(&config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&config_path, json_content)?;

        // On Unix, restrict permissions to 0600 (owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&config_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        Ok(key)
    }
}

/// Encrypts plaintext using AES-256-GCM
/// Output format: Base64([12-byte nonce] + [ciphertext + 16-byte tag])
pub fn encrypt_string(plaintext: &str, key: &[u8; 32]) -> Result<String, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| CryptoError::InvalidKeyLength(key.len()))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| CryptoError::Encryption)?;

    let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(combined))
}

/// Decrypts Base64 ciphertext using AES-256-GCM
pub fn decrypt_string(ciphertext_b64: &str, key: &[u8; 32]) -> Result<String, CryptoError> {
    let combined = BASE64.decode(ciphertext_b64)?;
    if combined.len() < 12 + 16 {
        return Err(CryptoError::CiphertextTooShort);
    }

    let (nonce_bytes, encrypted_bytes) = combined.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| CryptoError::InvalidKeyLength(key.len()))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let decrypted_bytes = cipher
        .decrypt(nonce, encrypted_bytes)
        .map_err(|_| CryptoError::Decryption)?;

    String::from_utf8(decrypted_bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        let secret_payload = r#"{"midtrans_server_key":"SB-Mid-server-XXXXX","google_token":"ya29.XXXX"}"#;
        let encrypted = encrypt_string(secret_payload, &key).expect("Encryption failed");

        assert_ne!(secret_payload, encrypted);

        let decrypted = decrypt_string(&encrypted, &key).expect("Decryption failed");
        assert_eq!(secret_payload, decrypted);
    }

    #[test]
    fn test_tamper_detection() {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        let encrypted = encrypt_string("confidential_data", &key).unwrap();
        let mut bytes = BASE64.decode(&encrypted).unwrap();
        // Tamper with one byte of ciphertext
        let len = bytes.len();
        bytes[len - 1] ^= 0xFF;
        let tampered_b64 = BASE64.encode(bytes);

        assert!(decrypt_string(&tampered_b64, &key).is_err());
    }
}
