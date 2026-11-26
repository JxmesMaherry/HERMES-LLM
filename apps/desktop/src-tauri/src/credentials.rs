use crate::{
    config::{self, SERVICE},
    error::{message, Result},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const VAULT_ACCOUNT: &str = "workspace-credentials-v1";
const LOCKED: &str = "Unlock saved credentials once using the Unlock button. Background checks will not retry Keychain access.";

// All secrets share one Keychain item. Only this module performs credential I/O.
// Cached access and health checks cannot call the operating-system credential API.
#[derive(Clone, Serialize, Deserialize)]
struct Secrets {
    backend_secret: String,
    tokens: BTreeMap<u16, String>,
}
impl Secrets {
    fn decode(value: &str) -> Result<Self> {
        if value.len() > 128 * 1024 {
            return Err(message(
                "Saved credentials exceed the supported size. They were not replaced.",
            ));
        }
        let secrets: Self = serde_json::from_str(value)
            .map_err(|_| message("Saved credentials could not be read. They were not replaced."))?;
        if secrets.backend_secret.len() < 24
            || secrets.backend_secret.len() > 1024
            || secrets.tokens.len() > 128
            || secrets
                .tokens
                .values()
                .any(|t| t.is_empty() || t.len() > 8192)
        {
            return Err(message(
                "Saved credentials are invalid. They were not replaced.",
            ));
        }
        Ok(secrets)
    }
}
pub trait Storage {
    fn read(&self, account: &str, interactive: bool) -> Result<Option<String>>;
    fn write(&self, account: &str, value: &str) -> Result<()>;
}
pub struct Keychain;
impl Storage for Keychain {
    fn read(&self, account: &str, interactive: bool) -> Result<Option<String>> {
        if interactive {
            return match config::credential(account)?.get_password() {
                Ok(value) => Ok(Some(value)), Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => Err(message("Keychain access was not granted. Click Unlock to retry when ready; no background retry will occur.")),
            };
        }
        // kSecUseAuthenticationUISkip explicitly forbids OS authentication dialogs.
        // Protected/inaccessible items are left alone until a deliberate Unlock action.
        use security_framework::item::{ItemClass, ItemSearchOptions, SearchResult};
        match ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(SERVICE)
            .account(account)
            .load_data(true)
            .skip_authenticated_items(true)
            .search()
        {
            Ok(items) => Ok(items.into_iter().find_map(|item| match item {
                SearchResult::Data(data) => String::from_utf8(data).ok(),
                _ => None,
            })),
            Err(error) if error.code() == -25300 => Ok(None),
            Err(_) => Err(message(LOCKED)),
        }
    }
    fn write(&self, account: &str, value: &str) -> Result<()> {
        config::credential(account)?.set_password(value).map_err(|_| message("Could not save credentials in Keychain. Your existing credentials were not replaced."))
    }
}
pub struct Vault<S: Storage = Keychain> {
    storage: S,
    secrets: Option<Secrets>,
}
impl<S: Storage> Vault<S> {
    pub fn restore_silently(storage: S) -> Self {
        let secrets = storage
            .read(VAULT_ACCOUNT, false)
            .ok()
            .flatten()
            .and_then(|v| Secrets::decode(&v).ok());
        Self { storage, secrets }
    }
    pub fn unlocked(&self) -> bool {
        self.secrets.is_some()
    }
    pub fn token(&self, port: u16) -> Result<String> {
        self.secrets
            .as_ref()
            .ok_or_else(|| message(LOCKED))?
            .tokens
            .get(&port)
            .cloned()
            .ok_or_else(|| {
                message("Sign in to your local Open WebUI account in Setup or Settings.")
            })
    }
    pub fn backend_secret(&self) -> Result<String> {
        Ok(self
            .secrets
            .as_ref()
            .ok_or_else(|| message(LOCKED))?
            .backend_secret
            .clone())
    }
    pub fn unlock(&mut self, port: u16, running_secret: Option<String>) -> Result<()> {
        if self.unlocked() {
            return Ok(());
        }
        if let Some(value) = self.storage.read(VAULT_ACCOUNT, true)? {
            self.secrets = Some(Secrets::decode(&value)?);
            return Ok(());
        }
        let mut tokens = BTreeMap::new();
        let backend_secret = if let Some(secret) = running_secret {
            // Existing installations: recover the unchanged secret from our own running
            // container, so only the old session item can need an interactive read.
            if let Some(token) = self.storage.read(&format!("webui-token-{port}"), true)? {
                tokens.insert(port, token);
            }
            secret
        } else {
            let secret = match self.storage.read("backend-secret", true)? {
                Some(secret) => secret,
                None => {
                    use rand::RngCore;
                    let mut bytes = [0u8; 32];
                    rand::rngs::OsRng.fill_bytes(&mut bytes);
                    bytes.iter().map(|b| format!("{b:02x}")).collect()
                }
            };
            // A legacy session is migrated only when it can be read WITHOUT a dialog.
            if let Ok(Some(token)) = self.storage.read(&format!("webui-token-{port}"), false) {
                tokens.insert(port, token);
            }
            secret
        };
        let secrets = Secrets {
            backend_secret,
            tokens,
        };
        let encoded = serde_json::to_string(&secrets)?;
        Secrets::decode(&encoded)?;
        self.storage.write(VAULT_ACCOUNT, &encoded)?;
        self.secrets = Some(secrets);
        Ok(())
    }
    pub fn set_token(&mut self, port: u16, token: Option<String>) -> Result<()> {
        let mut secrets = self.secrets.clone().ok_or_else(|| message(LOCKED))?;
        if let Some(token) = token {
            secrets.tokens.insert(port, token);
        } else {
            secrets.tokens.remove(&port);
        }
        let encoded = serde_json::to_string(&secrets)?;
        Secrets::decode(&encoded)?;
        self.storage.write(VAULT_ACCOUNT, &encoded)?;
        self.secrets = Some(secrets);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    #[derive(Clone, Default)]
    struct Fake {
        entries: Rc<RefCell<BTreeMap<String, String>>>,
        reads: Rc<RefCell<Vec<(String, bool)>>>,
        writes: Rc<Cell<usize>>,
        deny: Rc<Cell<bool>>,
    }
    impl Storage for Fake {
        fn read(&self, account: &str, interactive: bool) -> Result<Option<String>> {
            self.reads.borrow_mut().push((account.into(), interactive));
            if self.deny.get() {
                return Err(message("Denied"));
            }
            Ok(self.entries.borrow().get(account).cloned())
        }
        fn write(&self, account: &str, value: &str) -> Result<()> {
            self.writes.set(self.writes.get() + 1);
            self.entries
                .borrow_mut()
                .insert(account.into(), value.into());
            Ok(())
        }
    }
    fn encoded() -> String {
        serde_json::to_string(&Secrets {
            backend_secret: "a".repeat(64),
            tokens: BTreeMap::from([(3000, "session-token".into())]),
        })
        .unwrap()
    }
    #[test]
    fn background_checks_never_trigger_interactive_reads_or_retries() {
        let store = Fake::default();
        store.deny.set(true);
        let vault = Vault::restore_silently(store.clone());
        for _ in 0..100 {
            assert!(!vault.unlocked());
            assert!(vault.token(3000).is_err());
            assert!(vault.backend_secret().is_err());
        }
        assert_eq!(
            store.reads.borrow().as_slice(),
            &[(VAULT_ACCOUNT.into(), false)]
        );
        assert_eq!(store.writes.get(), 0);
    }
    #[test]
    fn successful_unlock_caches_every_secret_for_the_entire_session() {
        let store = Fake::default();
        let mut vault = Vault::restore_silently(store.clone());
        store
            .entries
            .borrow_mut()
            .insert(VAULT_ACCOUNT.into(), encoded());
        vault.unlock(3000, None).unwrap();
        let reads = store.reads.borrow().len();
        for _ in 0..100 {
            vault.unlock(3000, None).unwrap();
            assert_eq!(vault.token(3000).unwrap(), "session-token");
            assert_eq!(vault.backend_secret().unwrap(), "a".repeat(64));
        }
        assert_eq!(store.reads.borrow().len(), reads);
    }
    #[test]
    fn denied_unlock_is_latched_until_an_explicit_retry() {
        let store = Fake::default();
        store.deny.set(true);
        let mut vault = Vault::restore_silently(store.clone());
        assert!(vault.unlock(3000, None).is_err());
        let reads = store.reads.borrow().len();
        for _ in 0..100 {
            assert!(vault.token(3000).is_err());
        }
        assert_eq!(store.reads.borrow().len(), reads);
        assert_eq!(store.writes.get(), 0);
        store.deny.set(false);
        store
            .entries
            .borrow_mut()
            .insert(VAULT_ACCOUNT.into(), encoded());
        vault.unlock(3000, None).unwrap();
        assert!(vault.unlocked());
    }
    #[test]
    fn migration_keeps_the_backend_secret_and_uses_only_one_legacy_prompt() {
        let store = Fake::default();
        store
            .entries
            .borrow_mut()
            .insert("webui-token-3000".into(), "old-token".into());
        let mut vault = Vault::restore_silently(store.clone());
        vault.unlock(3000, Some("b".repeat(64))).unwrap();
        assert_eq!(vault.backend_secret().unwrap(), "b".repeat(64));
        assert_eq!(vault.token(3000).unwrap(), "old-token");
        let reads = store.reads.borrow();
        assert!(!reads.iter().any(|(a, _)| a == "backend-secret"));
        assert_eq!(
            reads
                .iter()
                .filter(|(a, i)| a.starts_with("webui-token") && *i)
                .count(),
            1
        );
    }
    #[test]
    fn sign_out_clears_only_the_session_and_persists_it() {
        let store = Fake::default();
        store
            .entries
            .borrow_mut()
            .insert(VAULT_ACCOUNT.into(), encoded());
        let mut vault = Vault::restore_silently(store.clone());
        vault.set_token(3000, None).unwrap();
        assert!(vault.token(3000).is_err());
        assert_eq!(vault.backend_secret().unwrap(), "a".repeat(64));
        let restored = Vault::restore_silently(store);
        assert!(restored.token(3000).is_err());
        assert!(restored.backend_secret().is_ok());
    }
}

#[cfg(test)]
mod corrupt_tests {
    use super::*;
    struct Corrupt;
    impl Storage for Corrupt {
        fn read(&self, _: &str, _: bool) -> Result<Option<String>> {
            Ok(Some("invalid saved data".into()))
        }
        fn write(&self, _: &str, _: &str) -> Result<()> {
            panic!("Corrupt credentials must never be overwritten automatically")
        }
    }
    #[test]
    fn invalid_saved_credentials_remain_locked_and_are_preserved() {
        let mut vault = Vault::restore_silently(Corrupt);
        assert!(!vault.unlocked());
        assert!(vault.unlock(3000, None).is_err());
        assert!(vault.backend_secret().is_err());
    }
}
