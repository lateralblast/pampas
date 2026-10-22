//! Where API keys come from: the environment variable a provider names, else the OS keychain
//! (macOS Keychain, or the Secret Service on Linux). Keys are stored under the same name as the
//! environment variable, e.g. `GROQ_API_KEY`.

use anyhow::Context;

const SERVICE: &str = "pampas";

/// Somewhere keys can be stored by name.
pub trait KeyStore {
    fn get(&self, name: &str) -> anyhow::Result<Option<String>>;
    fn set(&self, name: &str, key: &str) -> anyhow::Result<()>;
    /// Returns whether a key was there to delete.
    fn delete(&self, name: &str) -> anyhow::Result<bool>;
}

/// The operating system's native credential store.
pub struct Keychain;

fn entry(name: &str) -> anyhow::Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).context("the OS keychain is not available")
}

impl KeyStore for Keychain {
    fn get(&self, name: &str) -> anyhow::Result<Option<String>> {
        match entry(name)?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).context("reading from the OS keychain"),
        }
    }

    fn set(&self, name: &str, key: &str) -> anyhow::Result<()> {
        entry(name)?.set_password(key).context("writing to the OS keychain")
    }

    fn delete(&self, name: &str) -> anyhow::Result<bool> {
        match entry(name)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(e).context("deleting from the OS keychain"),
        }
    }
}

/// Where a key was found.
#[derive(Debug, PartialEq)]
pub enum Source {
    Env,
    Keychain,
}

/// Find the key called `name`: `env` (the value of the environment variable, if set) wins over the
/// store. An empty environment value counts as unset. The store is only consulted when needed, so
/// a missing keychain never breaks a run that has its keys in the environment.
pub fn resolve(name: &str, env: Option<String>, store: &dyn KeyStore) -> anyhow::Result<Option<(String, Source)>> {
    if let Some(key) = env.filter(|k| !k.is_empty()) {
        return Ok(Some((key, Source::Env)));
    }
    Ok(store.get(name)?.map(|key| (key, Source::Keychain)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Memory(RefCell<HashMap<String, String>>);

    impl KeyStore for Memory {
        fn get(&self, name: &str) -> anyhow::Result<Option<String>> {
            Ok(self.0.borrow().get(name).cloned())
        }
        fn set(&self, name: &str, key: &str) -> anyhow::Result<()> {
            self.0.borrow_mut().insert(name.into(), key.into());
            Ok(())
        }
        fn delete(&self, name: &str) -> anyhow::Result<bool> {
            Ok(self.0.borrow_mut().remove(name).is_some())
        }
    }

    /// A store that fails every call, like a Linux box with no Secret Service running.
    struct Broken;

    impl KeyStore for Broken {
        fn get(&self, _: &str) -> anyhow::Result<Option<String>> {
            anyhow::bail!("no keychain")
        }
        fn set(&self, _: &str, _: &str) -> anyhow::Result<()> {
            anyhow::bail!("no keychain")
        }
        fn delete(&self, _: &str) -> anyhow::Result<bool> {
            anyhow::bail!("no keychain")
        }
    }

    #[test]
    fn env_wins_over_store() {
        let store = Memory::default();
        store.set("K", "stored").unwrap();
        assert_eq!(resolve("K", Some("env".into()), &store).unwrap(), Some(("env".into(), Source::Env)));
    }

    #[test]
    fn store_is_used_when_env_is_unset_or_empty() {
        let store = Memory::default();
        store.set("K", "stored").unwrap();
        let want = Some(("stored".to_string(), Source::Keychain));
        assert_eq!(resolve("K", None, &store).unwrap(), want);
        assert_eq!(resolve("K", Some(String::new()), &store).unwrap(), want);
    }

    #[test]
    fn missing_everywhere_is_none() {
        assert_eq!(resolve("K", None, &Memory::default()).unwrap(), None);
    }

    #[test]
    fn broken_store_does_not_matter_when_env_is_set() {
        assert!(resolve("K", Some("env".into()), &Broken).unwrap().is_some());
        assert!(resolve("K", None, &Broken).is_err());
    }

    #[test]
    fn delete_reports_whether_a_key_existed() {
        let store = Memory::default();
        store.set("K", "v").unwrap();
        assert!(store.delete("K").unwrap());
        assert!(!store.delete("K").unwrap());
    }
}
