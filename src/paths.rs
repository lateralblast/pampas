//! Where pampas keeps its files on each platform, shared by the CLI and the GUI.
//!
//! - Linux: config and settings in `$XDG_CONFIG_HOME` or `~/.config`, usage in `$XDG_STATE_HOME` or
//!   `~/.local/state`, each under `pampas/`.
//! - macOS: `~/Library/Application Support/pampas` for everything. Older versions used the Linux
//!   locations, so those are still read until `pampas migrate` moves the files.
//! - Windows: `%APPDATA%\pampas` for config and settings, `%LOCALAPPDATA%\pampas` for usage.
//!
//! On Linux and macOS an `XDG_*` variable that is set always wins, which also lets tests and scratch
//! runs use a private directory. The functions take the OS and the environment as arguments so every
//! platform's rules can be tested from any machine; `Os::current()` and `std::env::var_os` are the
//! real ones.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(windows) {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

/// Looks up an environment variable; an empty value counts as unset.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<OsString>;

/// The real environment.
pub fn real_env(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

fn var(env: Env, name: &str) -> Option<PathBuf> {
    env(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn home(env: Env, os: Os) -> anyhow::Result<PathBuf> {
    let name = if os == Os::Windows { "USERPROFILE" } else { "HOME" };
    var(env, name).ok_or_else(|| anyhow!("{name} is not set"))
}

/// Directory of `providers.json` and `settings.json`.
pub fn config_dir(os: Os, env: Env) -> anyhow::Result<PathBuf> {
    let base = match os {
        Os::Windows => match var(env, "APPDATA") {
            Some(dir) => dir,
            None => home(env, os)?.join("AppData").join("Roaming"),
        },
        _ => match var(env, "XDG_CONFIG_HOME") {
            Some(dir) => dir,
            None if os == Os::MacOs => home(env, os)?.join("Library").join("Application Support"),
            None => home(env, os)?.join(".config"),
        },
    };
    Ok(base.join("pampas"))
}

/// Directory of `usage.json`.
pub fn state_dir(os: Os, env: Env) -> anyhow::Result<PathBuf> {
    let base = match os {
        Os::Windows => match var(env, "LOCALAPPDATA") {
            Some(dir) => dir,
            None => home(env, os)?.join("AppData").join("Local"),
        },
        _ => match var(env, "XDG_STATE_HOME") {
            Some(dir) => dir,
            None if os == Os::MacOs => home(env, os)?.join("Library").join("Application Support"),
            None => home(env, os)?.join(".local").join("state"),
        },
    };
    Ok(base.join("pampas"))
}

/// Where older versions kept the config on macOS (the Linux location), if it differs from the current one.
fn legacy_config_dir(os: Os, env: Env) -> Option<PathBuf> {
    (os == Os::MacOs && var(env, "XDG_CONFIG_HOME").is_none()).then(|| home(env, os).ok()).flatten().map(|h| h.join(".config").join("pampas"))
}

/// Where older versions kept the usage file on macOS, if it differs from the current one.
fn legacy_state_dir(os: Os, env: Env) -> Option<PathBuf> {
    (os == Os::MacOs && var(env, "XDG_STATE_HOME").is_none())
        .then(|| home(env, os).ok())
        .flatten()
        .map(|h| h.join(".local").join("state").join("pampas"))
}

/// A default file path, and whether it is an old-location file being used until `pampas migrate` runs.
#[derive(Debug, PartialEq)]
pub struct Located {
    pub path: PathBuf,
    pub legacy: bool,
}

fn locate(current: PathBuf, legacy: Option<PathBuf>, file: &str, exists: &dyn Fn(&Path) -> bool) -> Located {
    let new = current.join(file);
    if let Some(old) = legacy.map(|d| d.join(file))
        && !exists(&new)
        && exists(&old)
    {
        return Located { path: old, legacy: true };
    }
    Located { path: new, legacy: false }
}

/// The default providers config path.
pub fn config_file(os: Os, env: Env, exists: &dyn Fn(&Path) -> bool) -> anyhow::Result<Located> {
    Ok(locate(config_dir(os, env)?, legacy_config_dir(os, env), "providers.json", exists))
}

/// The default usage file path.
pub fn usage_file(os: Os, env: Env, exists: &dyn Fn(&Path) -> bool) -> anyhow::Result<Located> {
    Ok(locate(state_dir(os, env)?, legacy_state_dir(os, env), "usage.json", exists))
}

/// Old-location files that exist and have no counterpart at the new location: `(from, to)` pairs.
pub fn migration_plan(os: Os, env: Env, exists: &dyn Fn(&Path) -> bool) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
    let mut plan = Vec::new();
    for (legacy, current, files) in [
        (legacy_config_dir(os, env), config_dir(os, env)?, ["providers.json", "settings.json"].as_slice()),
        (legacy_state_dir(os, env), state_dir(os, env)?, ["usage.json"].as_slice()),
    ] {
        let Some(legacy) = legacy else { continue };
        for file in files {
            let (from, to) = (legacy.join(file), current.join(file));
            if exists(&from) && !exists(&to) {
                plan.push((from, to));
            }
        }
    }
    Ok(plan)
}

/// Move the files in `plan`, creating directories as needed. Returns the number moved.
pub fn migrate(plan: &[(PathBuf, PathBuf)]) -> anyhow::Result<usize> {
    for (from, to) in plan {
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        if std::fs::rename(from, to).is_err() {
            // Different filesystems cannot be renamed across: copy, then remove the original.
            std::fs::copy(from, to).with_context(|| format!("copying {} to {}", from.display(), to.display()))?;
            std::fs::remove_file(from).with_context(|| format!("removing {}", from.display()))?;
        }
    }
    Ok(plan.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |name| map.get(name).map(OsString::from)
    }

    fn p(parts: &[&str]) -> PathBuf {
        parts.iter().collect()
    }

    #[test]
    fn linux_uses_xdg_locations() {
        let e = env(&[("HOME", "/home/u")]);
        assert_eq!(config_dir(Os::Linux, &e).unwrap(), p(&["/home/u", ".config", "pampas"]));
        assert_eq!(state_dir(Os::Linux, &e).unwrap(), p(&["/home/u", ".local", "state", "pampas"]));
        let e = env(&[("HOME", "/home/u"), ("XDG_CONFIG_HOME", "/x/c"), ("XDG_STATE_HOME", "/x/s")]);
        assert_eq!(config_dir(Os::Linux, &e).unwrap(), p(&["/x/c", "pampas"]));
        assert_eq!(state_dir(Os::Linux, &e).unwrap(), p(&["/x/s", "pampas"]));
    }

    #[test]
    fn macos_uses_application_support_unless_xdg_is_set() {
        let e = env(&[("HOME", "/Users/u")]);
        let want = p(&["/Users/u", "Library", "Application Support", "pampas"]);
        assert_eq!(config_dir(Os::MacOs, &e).unwrap(), want);
        assert_eq!(state_dir(Os::MacOs, &e).unwrap(), want);
        let e = env(&[("HOME", "/Users/u"), ("XDG_CONFIG_HOME", "/x/c")]);
        assert_eq!(config_dir(Os::MacOs, &e).unwrap(), p(&["/x/c", "pampas"]));
    }

    #[test]
    fn windows_uses_appdata_and_needs_no_home() {
        let e = env(&[("APPDATA", r"C:\Users\u\AppData\Roaming"), ("LOCALAPPDATA", r"C:\Users\u\AppData\Local")]);
        assert_eq!(config_dir(Os::Windows, &e).unwrap(), p(&[r"C:\Users\u\AppData\Roaming", "pampas"]));
        assert_eq!(state_dir(Os::Windows, &e).unwrap(), p(&[r"C:\Users\u\AppData\Local", "pampas"]));
        // Without them, fall back to the profile directory; XDG variables are ignored on Windows.
        let e = env(&[("USERPROFILE", r"C:\Users\u"), ("XDG_CONFIG_HOME", "/ignored")]);
        assert_eq!(config_dir(Os::Windows, &e).unwrap(), p(&[r"C:\Users\u", "AppData", "Roaming", "pampas"]));
        assert_eq!(state_dir(Os::Windows, &e).unwrap(), p(&[r"C:\Users\u", "AppData", "Local", "pampas"]));
    }

    #[test]
    fn a_missing_home_is_an_error_naming_it() {
        let none = env(&[]);
        assert!(config_dir(Os::Linux, &none).unwrap_err().to_string().contains("HOME"));
        assert!(config_dir(Os::Windows, &none).unwrap_err().to_string().contains("USERPROFILE"));
        assert!(config_dir(Os::Linux, &env(&[("HOME", "")])).is_err(), "empty counts as unset");
    }

    #[test]
    fn macos_falls_back_to_the_old_location_until_migrated() {
        let e = env(&[("HOME", "/Users/u")]);
        let old = p(&["/Users/u", ".config", "pampas", "providers.json"]);
        let new = p(&["/Users/u", "Library", "Application Support", "pampas", "providers.json"]);
        let only_old = |path: &Path| path == old;
        assert_eq!(config_file(Os::MacOs, &e, &only_old).unwrap(), Located { path: old.clone(), legacy: true });
        let both = |path: &Path| path == old || path == new;
        assert_eq!(config_file(Os::MacOs, &e, &both).unwrap(), Located { path: new.clone(), legacy: false });
        let neither = |_: &Path| false;
        assert_eq!(config_file(Os::MacOs, &e, &neither).unwrap(), Located { path: new, legacy: false });
        // Linux never has an old location.
        let linux = env(&[("HOME", "/home/u")]);
        assert!(!config_file(Os::Linux, &linux, &|_: &Path| true).unwrap().legacy);
    }

    #[test]
    fn migration_moves_only_files_that_have_no_counterpart() {
        let e = env(&[("HOME", "/Users/u")]);
        let old_cfg = p(&["/Users/u", ".config", "pampas"]);
        let new_dir = p(&["/Users/u", "Library", "Application Support", "pampas"]);
        let existing = [old_cfg.join("providers.json"), old_cfg.join("settings.json"), new_dir.join("settings.json")];
        let plan = migration_plan(Os::MacOs, &e, &|path: &Path| existing.iter().any(|f| f == path)).unwrap();
        assert_eq!(plan, vec![(old_cfg.join("providers.json"), new_dir.join("providers.json"))]);
        assert!(migration_plan(Os::Linux, &env(&[("HOME", "/h")]), &|_: &Path| true).unwrap().is_empty());
        // An explicit XDG variable means the old and new locations are the same thing.
        let xdg = env(&[("HOME", "/Users/u"), ("XDG_CONFIG_HOME", "/x"), ("XDG_STATE_HOME", "/y")]);
        assert!(migration_plan(Os::MacOs, &xdg, &|_: &Path| true).unwrap().is_empty());
    }

    #[test]
    fn migrate_moves_files_on_disk() {
        let dir = std::env::temp_dir().join(format!("pampas-paths-test-{}", std::process::id()));
        let (from, to) = (dir.join("old").join("a.json"), dir.join("new").join("sub").join("a.json"));
        std::fs::create_dir_all(from.parent().unwrap()).unwrap();
        std::fs::write(&from, "[]").unwrap();
        assert_eq!(migrate(&[(from.clone(), to.clone())]).unwrap(), 1);
        assert!(!from.exists());
        assert_eq!(std::fs::read_to_string(&to).unwrap(), "[]");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
