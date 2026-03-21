use std::{fs, path::Path};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const KEY_DERIVE_SALT: &[u8] = b"slimjelly-machine-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionToken {
    pub access_token: String,
    pub user_id: String,
    pub server_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedBlob {
    nonce_b64: String,
    ciphertext_b64: String,
}

pub fn store_session(path: &Path, token: &SessionToken) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let key = derive_machine_key()?;
    let cipher = XChaCha20Poly1305::new(&key);

    let mut nonce_bytes = [0_u8; 24];
    getrandom::fill(&mut nonce_bytes)
        .map_err(|err| AppError::Crypto(format!("nonce generation failed: {err}")))?;

    let payload = serde_json::to_vec(token)?;
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, payload.as_ref())?;

    let blob = EncryptedBlob {
        nonce_b64: STANDARD.encode(nonce_bytes),
        ciphertext_b64: STANDARD.encode(ciphertext),
    };

    let output = serde_json::to_vec_pretty(&blob)?;
    fs::write(path, output)?;
    Ok(())
}

pub fn load_session(path: &Path) -> Result<Option<SessionToken>, AppError> {
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read(path)?;
    let blob: EncryptedBlob = serde_json::from_slice(&raw)?;
    let nonce = STANDARD
        .decode(blob.nonce_b64)
        .map_err(|err| AppError::Crypto(format!("invalid nonce encoding: {err}")))?;
    let ciphertext = STANDARD
        .decode(blob.ciphertext_b64)
        .map_err(|err| AppError::Crypto(format!("invalid ciphertext encoding: {err}")))?;

    if nonce.len() != 24 {
        return Err(AppError::Crypto(
            "invalid nonce length in encrypted session".to_string(),
        ));
    }

    let key = derive_machine_key()?;
    let cipher = XChaCha20Poly1305::new(&key);

    let plaintext = cipher.decrypt(XNonce::from_slice(&nonce), ciphertext.as_ref())?;
    let token = serde_json::from_slice::<SessionToken>(&plaintext)?;
    Ok(Some(token))
}

pub fn clear_session(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn derive_machine_key() -> Result<Key, AppError> {
    let machine_identity = machine_identity()?;
    let params = Params::new(32 * 1024, 3, 1, Some(32))
        .map_err(|err| AppError::Crypto(format!("argon2 params error: {err}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut out = [0_u8; 32];
    argon
        .hash_password_into(machine_identity.as_bytes(), KEY_DERIVE_SALT, &mut out)
        .map_err(|err| AppError::Crypto(format!("key derivation failed: {err}")))?;

    Ok(*Key::from_slice(&out))
}

fn machine_identity() -> Result<String, AppError> {
    let host = hostname::get()
        .map_err(|err| AppError::Crypto(format!("hostname read failed: {err}")))?
        .to_string_lossy()
        .to_string();

    let machine_id = fs::read_to_string("/etc/machine-id")
        .or_else(|_| fs::read_to_string("/var/lib/dbus/machine-id"))
        .unwrap_or_else(|_| "unknown-machine-id".to_string());

    Ok(format!("{host}:{}", machine_id.trim()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use uuid::Uuid;

    use super::*;

    fn temp_session_path(test_name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("slimjelly-tests")
            .join(format!("{test_name}-{}", Uuid::new_v4()))
            .join("session.enc")
    }

    fn cleanup(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn stores_and_loads_session_roundtrip() -> Result<(), AppError> {
        let path = temp_session_path("roundtrip");
        let token = SessionToken {
            access_token: "token-123".to_string(),
            user_id: "user-456".to_string(),
            server_id: Some("server-789".to_string()),
        };

        store_session(&path, &token)?;
        let loaded = load_session(&path)?.expect("stored session must be present");
        assert_eq!(loaded, token);

        clear_session(&path)?;
        assert!(load_session(&path)?.is_none());

        cleanup(&path);
        Ok(())
    }

    #[test]
    fn returns_error_for_invalid_nonce_length_blob() {
        let path = temp_session_path("invalid-nonce-length");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("must create temp parent dir");
        }

        fs::write(
            &path,
            br#"{"nonce_b64":"AA==","ciphertext_b64":"AA=="}"#,
        )
        .expect("must write corrupt blob");

        let result = load_session(&path);
        assert!(matches!(
            result,
            Err(AppError::Crypto(message)) if message.contains("nonce length")
        ));

        cleanup(&path);
    }

    #[test]
    fn clear_session_is_idempotent_for_missing_file() -> Result<(), AppError> {
        let path = temp_session_path("clear-missing");
        clear_session(&path)?;
        clear_session(&path)?;
        cleanup(&path);
        Ok(())
    }
}
