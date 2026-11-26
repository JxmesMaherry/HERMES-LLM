use crate::{
    config,
    error::{message, Result},
};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use zeroize::{Zeroize, Zeroizing};

const AAD: &[u8] = b"HERMES-LLM credentials v1";
const MAX_FILE: u64 = 256 * 1024;
const LOCKED: &str =
    "Unlock HERMES with the password you chose. Your Mac or Keychain password is not used.";
#[derive(Clone, Serialize, Deserialize)]
struct Secrets {
    backend_secret: String,
    tokens: BTreeMap<u16, String>,
}
impl Drop for Secrets {
    fn drop(&mut self) {
        self.backend_secret.zeroize();
        for token in self.tokens.values_mut() {
            token.zeroize();
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u8,
    kdf: String,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    salt: Vec<u8>,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub exists: bool,
    pub unlocked: bool,
}
pub struct Vault {
    path: PathBuf,
    secrets: Option<Secrets>,
    key: Option<Zeroizing<[u8; 32]>>,
    salt: Option<[u8; 16]>,
}
fn derive(password: &str, salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(65536, 3, 1, Some(32))
        .map_err(|_| message("Could not initialise password protection."))?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut key[..])
        .map_err(|_| message("Could not unlock password protection."))?;
    Ok(key)
}
pub fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err(message(
            "Choose a HERMES password with at least 12 characters (maximum 1024 bytes).",
        ));
    }
    Ok(())
}
impl Vault {
    pub fn open(path: PathBuf) -> Self {
        Self {
            path,
            secrets: None,
            key: None,
            salt: None,
        }
    }
    pub fn status(&self) -> Status {
        Status {
            exists: self.path.exists(),
            unlocked: self.unlocked(),
        }
    }
    pub fn unlocked(&self) -> bool {
        self.secrets.is_some()
    }
    pub fn token(&self, port: u16) -> Result<String> {
        self.secrets.as_ref().ok_or_else(||message(LOCKED))?.tokens.get(&port).cloned().ok_or_else(||message("Connect your local account in Settings. Use account recovery if you forgot its password."))
    }
    pub fn backend_secret(&self) -> Result<String> {
        Ok(self
            .secrets
            .as_ref()
            .ok_or_else(|| message(LOCKED))?
            .backend_secret
            .clone())
    }
    fn envelope(&self) -> Result<Envelope> {
        if std::fs::symlink_metadata(&self.path)?
            .file_type()
            .is_symlink()
            || std::fs::metadata(&self.path)?.len() > MAX_FILE
        {
            return Err(message(
                "The saved credential vault is not a supported regular file.",
            ));
        }
        let envelope:Envelope=serde_json::from_slice(&std::fs::read(&self.path)?).map_err(|_|message("The credential vault is damaged. Use Reset saved sign-ins to recover without deleting books."))?;
        if envelope.version != 1
            || envelope.kdf != "argon2id"
            || envelope.memory_kib != 65536
            || envelope.iterations != 3
            || envelope.parallelism != 1
            || envelope.salt.len() != 16
            || envelope.nonce.len() != 24
            || envelope.ciphertext.len() > 128 * 1024
        {
            return Err(message(
                "Unsupported credential vault format. Existing data was not changed.",
            ));
        }
        Ok(envelope)
    }
    pub fn unlock(&mut self, password: &str) -> Result<()> {
        if self.unlocked() {
            return Ok(());
        }
        let envelope = self.envelope()?;
        let salt: [u8; 16] = envelope
            .salt
            .as_slice()
            .try_into()
            .map_err(|_| message("Invalid credential vault salt."))?;
        let nonce: [u8; 24] = envelope
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| message("Invalid credential vault nonce."))?;
        let key = derive(password, &salt)?;
        let cipher = XChaCha20Poly1305::new_from_slice(&key[..])
            .map_err(|_| message("Could not open credential vault."))?;
        let plaintext=Zeroizing::new(cipher.decrypt((&nonce).into(),Payload {msg:&envelope.ciphertext,aad:AAD}).map_err(|_|message("That HERMES password did not unlock the vault. Try again or use Reset saved sign-ins; your books will be kept."))?);
        let secrets: Secrets = serde_json::from_slice(&plaintext)
            .map_err(|_| message("Saved credentials could not be decoded."))?;
        if secrets.backend_secret.len() < 24
            || secrets.backend_secret.len() > 1024
            || secrets.tokens.len() > 128
            || secrets
                .tokens
                .values()
                .any(|v| v.is_empty() || v.len() > 8192)
        {
            return Err(message(
                "Saved credentials are invalid; no files were changed.",
            ));
        }
        self.secrets = Some(secrets);
        self.key = Some(key);
        self.salt = Some(salt);
        Ok(())
    }
    fn persist(&self, secrets: &Secrets, key: &[u8; 32], salt: &[u8; 16]) -> Result<()> {
        let mut nonce = [0u8; 24];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let plaintext = Zeroizing::new(serde_json::to_vec(secrets)?);
        if plaintext.len() > 48 * 1024 {
            return Err(message("Saved sessions exceed the vault size limit. Reset saved sign-ins to clear unused sessions."));
        }
        let cipher = XChaCha20Poly1305::new_from_slice(key)
            .map_err(|_| message("Could not protect saved credentials."))?;
        let ciphertext = cipher
            .encrypt(
                (&nonce).into(),
                Payload {
                    msg: &plaintext,
                    aad: AAD,
                },
            )
            .map_err(|_| message("Could not encrypt saved credentials."))?;
        let envelope = Envelope {
            version: 1,
            kdf: "argon2id".into(),
            memory_kib: 65536,
            iterations: 3,
            parallelism: 1,
            salt: salt.to_vec(),
            nonce: nonce.to_vec(),
            ciphertext,
        };
        config::atomic_write(&self.path, &serde_json::to_vec(&envelope)?)
    }
    pub fn create(&mut self, password: &str, backend_secret: String, reset: bool) -> Result<()> {
        validate_password(password)?;
        if backend_secret.len() < 24 || backend_secret.len() > 1024 {
            return Err(message("The backend secret could not be recovered safely."));
        }
        if self.path.exists() && !reset {
            return Err(message(
                "A HERMES password already exists. Unlock it or choose Reset saved sign-ins.",
            ));
        }
        let mut salt = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        let key = derive(password, &salt)?;
        let secrets = Secrets {
            backend_secret,
            tokens: BTreeMap::new(),
        };
        if self.path.exists()
            && (std::fs::symlink_metadata(&self.path)?
                .file_type()
                .is_symlink()
                || !std::fs::metadata(&self.path)?.is_file()
                || std::fs::metadata(&self.path)?.len() > MAX_FILE)
        {
            return Err(message("The existing credential vault is not a supported regular file; it was not replaced."));
        }
        // Recovery preserves an encrypted backup; it never touches documents or the backend DB.
        if self.path.exists() {
            let backup = self
                .path
                .with_file_name(format!("credentials-backup-{}.vault", uuid::Uuid::new_v4()));
            config::atomic_write(&backup, &std::fs::read(&self.path)?)?;
        }
        self.persist(&secrets, &key, &salt)?;
        self.secrets = Some(secrets);
        self.key = Some(key);
        self.salt = Some(salt);
        Ok(())
    }
    pub fn set_token(&mut self, port: u16, token: Option<String>) -> Result<()> {
        let mut secrets = self.secrets.clone().ok_or_else(|| message(LOCKED))?;
        if let Some(token) = token {
            if token.is_empty() || token.len() > 8192 {
                return Err(message("Invalid local session."));
            }
            secrets.tokens.insert(port, token);
        } else {
            secrets.tokens.remove(&port);
        }
        self.persist(
            &secrets,
            self.key.as_ref().ok_or_else(|| message(LOCKED))?,
            self.salt.as_ref().ok_or_else(|| message(LOCKED))?,
        )?;
        self.secrets = Some(secrets);
        Ok(())
    }
}
pub fn fresh_backend_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn path(root: &Path) -> PathBuf {
    root.join("credentials.vault")
}

#[cfg(test)]
mod tests {
    use super::*;
    const PASS: &str = "my new HERMES password";
    #[test]
    fn encryption_round_trip_and_cached_checks() {
        let d = tempfile::tempdir().unwrap();
        let file = path(d.path());
        let mut v = Vault::open(file.clone());
        assert!(!v.status().exists);
        v.create(PASS, "a".repeat(64), false).unwrap();
        v.set_token(3000, Some("private-session-value".into()))
            .unwrap();
        let bytes = std::fs::read(&file).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private-session-value"));
        assert!(!String::from_utf8_lossy(&bytes).contains(PASS));
        let mut restored = Vault::open(file);
        assert!(!restored.unlocked());
        restored.unlock(PASS).unwrap();
        for _ in 0..100 {
            assert_eq!(restored.token(3000).unwrap(), "private-session-value");
            assert_eq!(restored.backend_secret().unwrap(), "a".repeat(64));
        }
        restored.set_token(3000, None).unwrap();
        assert!(restored.token(3000).is_err());
    }
    #[test]
    fn wrong_password_and_tampering_preserve_vault() {
        let d = tempfile::tempdir().unwrap();
        let file = path(d.path());
        let mut v = Vault::open(file.clone());
        v.create(PASS, "b".repeat(64), false).unwrap();
        let saved = std::fs::read(&file).unwrap();
        let mut other = Vault::open(file.clone());
        assert!(other.unlock("wrong password").is_err());
        assert!(!other.unlocked());
        assert_eq!(std::fs::read(&file).unwrap(), saved);
        let mut e: Envelope = serde_json::from_slice(&saved).unwrap();
        e.ciphertext[0] ^= 1;
        std::fs::write(&file, serde_json::to_vec(&e).unwrap()).unwrap();
        assert!(other.unlock(PASS).is_err());
    }
    #[test]
    fn reset_backs_up_vault_and_keeps_backend_secret_and_documents() {
        let d = tempfile::tempdir().unwrap();
        let file = path(d.path());
        std::fs::write(d.path().join("book.txt"), "keep book").unwrap();
        let mut v = Vault::open(file.clone());
        v.create(PASS, "c".repeat(64), false).unwrap();
        v.set_token(3000, Some("old-session".into())).unwrap();
        let old = std::fs::read(&file).unwrap();
        assert!(v
            .create("a different password", "c".repeat(64), false)
            .is_err());
        v.create("a different password", "c".repeat(64), true)
            .unwrap();
        assert_eq!(v.backend_secret().unwrap(), "c".repeat(64));
        assert!(v.token(3000).is_err());
        assert!(std::fs::read_dir(d.path()).unwrap().flatten().any(|f| f
            .file_name()
            .to_string_lossy()
            .starts_with("credentials-backup-")
            && std::fs::read(f.path()).unwrap() == old));
        assert_eq!(
            std::fs::read_to_string(d.path().join("book.txt")).unwrap(),
            "keep book"
        );
    }
    #[test]
    fn hostile_parameters_and_weak_password_do_not_change_files() {
        let d = tempfile::tempdir().unwrap();
        let mut v = Vault::open(path(d.path()));
        assert!(v.create("short", "a".repeat(64), false).is_err());
        assert!(!v.status().exists);
        let e = Envelope {
            version: 1,
            kdf: "argon2id".into(),
            memory_kib: u32::MAX,
            iterations: 3,
            parallelism: 1,
            salt: vec![0; 16],
            nonce: vec![0; 24],
            ciphertext: vec![0; 20],
        };
        std::fs::write(&v.path, serde_json::to_vec(&e).unwrap()).unwrap();
        assert!(v.unlock(PASS).is_err());
    }
}
