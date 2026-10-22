//! Non-secret values for `${NAME}` placeholders in a provider's `base_url` (an account id, a host),
//! kept in `settings.json` beside the providers config. The environment variable of the same name
//! wins over a stored value. Secrets belong in the keychain (`pampas key set`), not here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;

/// Stored values by name.
#[derive(Debug, Default, PartialEq)]
pub struct Settings {
    values: BTreeMap<String, String>,
}

/// The settings file that goes with a providers config: `settings.json` in the same directory.
pub fn settings_path(config: &Path) -> PathBuf {
    config.with_file_name("settings.json")
}

/// A name must be usable as an environment variable and must not look like a secret.
pub fn check_name(name: &str) -> anyhow::Result<()> {
    let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    anyhow::ensure!(valid, "{name:?} is not a valid name: use letters, digits and underscores, e.g. CLOUDFLARE_ACCOUNT_ID");
    let upper = name.to_ascii_uppercase();
    anyhow::ensure!(
        !["_KEY", "_TOKEN", "_SECRET", "_PASSWORD"].iter().any(|s| upper.ends_with(s)),
        "{name} looks like a secret; settings are stored in plain text. Use `pampas key set {name}` instead"
    );
    Ok(())
}

impl Settings {
    /// Read the file; a missing file is an empty set.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let values = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
                Ok(Self { values })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(&self.values)? + "\n";
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned().filter(|v| !v.is_empty())
    }

    pub fn set(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        check_name(name)?;
        anyhow::ensure!(!value.trim().is_empty(), "no value given for {name}");
        self.values.insert(name.to_string(), value.trim().to_string());
        Ok(())
    }

    /// Returns whether the name was stored.
    pub fn delete(&mut self, name: &str) -> bool {
        self.values.remove(name).is_some()
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_delete_round_trip_through_a_file() {
        let dir = std::env::temp_dir().join(format!("pampas-settings-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        let mut s = Settings::load(&path).unwrap();
        assert_eq!(s, Settings::default());
        s.set("CLOUDFLARE_ACCOUNT_ID", " abc123 ").unwrap();
        s.save(&path).unwrap();
        let mut again = Settings::load(&path).unwrap();
        assert_eq!(again.get("CLOUDFLARE_ACCOUNT_ID").as_deref(), Some("abc123"));
        assert!(again.delete("CLOUDFLARE_ACCOUNT_ID"));
        assert!(!again.delete("CLOUDFLARE_ACCOUNT_ID"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn secrets_and_bad_names_are_refused() {
        let mut s = Settings::default();
        assert!(s.set("GROQ_API_KEY", "x").unwrap_err().to_string().contains("pampas key set"));
        assert!(s.set("MY_TOKEN", "x").is_err());
        assert!(s.set("bad name", "x").is_err());
        assert!(s.set("1ABC", "x").is_err());
        assert!(s.set("HOST", "  ").is_err());
    }

    #[test]
    fn settings_sit_beside_the_config() {
        assert_eq!(settings_path(Path::new("/a/b/providers.json")), PathBuf::from("/a/b/settings.json"));
    }
}
