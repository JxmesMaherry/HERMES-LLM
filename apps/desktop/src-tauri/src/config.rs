use crate::error::{message, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const SERVICE: &str = "com.jxmesmaherry.hermesllm";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub port: u16,
    pub model: String,
    pub folder: Option<PathBuf>,
    pub knowledge_id: Option<String>,
    pub theme: String,
    pub setup_complete: bool,
    pub allow_downloads: bool,
    pub max_file_mb: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3000,
            model: "qwen3:4b-instruct".into(),
            folder: None,
            knowledge_id: None,
            theme: "dark".into(),
            setup_complete: false,
            allow_downloads: false,
            max_file_mb: 20,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.port < 1024 {
            return Err(message("Choose a backend port from 1024 to 65535."));
        }
        if !valid_model(&self.model) {
            return Err(message("Enter a valid Ollama model name."));
        }
        if !["light", "dark"].contains(&self.theme.as_str()) {
            return Err(message("Theme must be light or dark."));
        }
        if !(1..=50).contains(&self.max_file_mb) {
            return Err(message("File size limit must be between 1 and 50 MB."));
        }
        if let Some(id) = &self.knowledge_id {
            validate_id(id)?;
        }
        Ok(())
    }
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}
pub fn valid_model(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 160
        && !name.starts_with(['-', '/', '.'])
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._:/-".contains(&c))
        && !name.contains("..")
}
pub fn validate_id(id: &str) -> Result<()> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| message("The backend returned an invalid resource identifier."))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    let mut file = options.open(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}
pub fn initialize(root: &Path, resource: &Path) -> Result<Config> {
    fs::create_dir_all(root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    }
    // Copy the bundled, versioned definition outside the read-only .app on every upgrade.
    atomic_write(&root.join("compose.yaml"), &fs::read(resource)?)?;
    let path = root.join("config.json");
    let config = if path.exists() {
        serde_json::from_slice(&fs::read(path)?)?
    } else {
        Config::default()
    };
    save(root, &config)?;
    Ok(config)
}
pub fn save(root: &Path, config: &Config) -> Result<()> {
    config.validate()?;
    atomic_write(
        &root.join("config.json"),
        &serde_json::to_vec_pretty(config)?,
    )
}
pub fn credential(account: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, account).map_err(|_| {
        message("macOS Keychain is unavailable. Unlock your login Keychain and retry.")
    })
}
pub fn token(port: u16) -> Result<String> {
    credential(&format!("webui-token-{port}"))?
        .get_password()
        .map_err(|_| message("Sign in to your local Open WebUI account in Setup or Settings."))
}
pub fn backend_secret() -> Result<String> {
    use rand::RngCore;
    let entry = credential("backend-secret")?;
    match entry.get_password() {
        Ok(secret) => Ok(secret),
        Err(keyring::Error::NoEntry) => {
            let mut bytes = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut bytes);
            let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            entry
                .set_password(&secret)
                .map_err(|_| message("Could not save the backend secret in Keychain."))?;
            Ok(secret)
        }
        Err(_) => Err(message(
            "Could not read the backend secret. Unlock your login Keychain.",
        )),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_config_and_identifiers() {
        let mut c = Config::default();
        assert!(c.validate().is_ok());
        c.port = 80;
        assert!(c.validate().is_err());
        c.port = 3000;
        c.model = "$(touch /tmp/no)".into();
        assert!(c.validate().is_err());
        assert!(!valid_model("../bad"));
        assert!(valid_model("qwen3:4b-instruct"));
        assert!(validate_id("../../x").is_err());
    }
    #[test]
    fn settings_survive_reload_without_secrets() {
        let d = tempfile::tempdir().unwrap();
        let resource = d.path().join("source.yaml");
        fs::write(&resource, "services: {}").unwrap();
        let root = d.path().join("app");
        let mut c = initialize(&root, &resource).unwrap();
        c.folder = Some(d.path().join("My Documents"));
        save(&root, &c).unwrap();
        assert_eq!(initialize(&root, &resource).unwrap().folder, c.folder);
        assert!(!fs::read_to_string(root.join("config.json"))
            .unwrap()
            .contains("token"));
    }
}
