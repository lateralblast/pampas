//! Counting requests per provider per UTC day, so free-tier limits are not exceeded.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The usage counts, shared between the providers running concurrently.
pub type SharedUsage = Arc<Mutex<Usage>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Counter {
    day: String,
    count: u32,
}

/// A request was refused because the daily limit is used up.
#[derive(Debug)]
pub struct LimitReached {
    pub group: String,
    pub limit: u32,
}

impl fmt::Display for LimitReached {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "daily limit of {} requests reached for `{}` (resets at 00:00 UTC)", self.limit, self.group)
    }
}

impl std::error::Error for LimitReached {}

/// Requests made today, by usage group, kept in a JSON file `{group: {day, count}}`.
pub struct Usage {
    path: Option<PathBuf>,
    today: String,
    counters: BTreeMap<String, Counter>,
}

impl Usage {
    /// Counts that are not saved anywhere.
    #[cfg(test)]
    pub fn in_memory(today: &str) -> Self {
        Self { path: None, today: today.to_string(), counters: BTreeMap::new() }
    }

    /// Load the counts saved at `path`. A missing file means no usage yet; an unreadable one is set
    /// aside as `<path>.corrupt` with a warning, and counting starts from zero.
    pub fn load(path: PathBuf, today: String) -> Self {
        let counters = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                let aside = path.with_extension("json.corrupt");
                eprintln!("warning: {} is unreadable ({e}); moved to {}, counting from zero", path.display(), aside.display());
                let _ = std::fs::rename(&path, aside);
                BTreeMap::new()
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => {
                eprintln!("warning: cannot read {} ({e}); counting from zero", path.display());
                BTreeMap::new()
            }
        };
        Self { path: Some(path), today, counters }
    }

    /// Requests made today under `group`.
    pub fn used(&self, group: &str) -> u32 {
        self.counters.get(group).filter(|c| c.day == self.today).map_or(0, |c| c.count)
    }

    /// Record one request under `group` and return today's count including it. With `enforce` and a
    /// `limit`, refuse (and do not count) once the limit is used up.
    pub fn acquire(&mut self, group: &str, limit: Option<u32>, enforce: bool) -> Result<u32, LimitReached> {
        let used = self.used(group);
        if let (Some(limit), true) = (limit, enforce)
            && used >= limit
        {
            return Err(LimitReached { group: group.to_string(), limit });
        }
        self.counters.insert(group.to_string(), Counter { day: self.today.clone(), count: used + 1 });
        self.save();
        Ok(used + 1)
    }

    /// Write the counts of today, atomically. A failure is a warning: it must not stop the run.
    fn save(&mut self) {
        let Some(path) = &self.path else { return };
        self.counters.retain(|_, c| c.day == self.today);
        let result = (|| -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_string_pretty(&self.counters).expect("counters serialize"))?;
            std::fs::rename(tmp, path)
        })();
        if let Err(e) = result {
            eprintln!("warning: cannot save request counts to {}: {e}", path.display());
        }
    }
}

/// Today's date in UTC as `YYYY-MM-DD`.
pub fn today_utc() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    civil_date((secs / 86_400) as i64)
}

/// The date `days` after 1970-01-01 (proleptic Gregorian calendar).
fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pampas-usage-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("usage.json")
    }

    #[test]
    fn dates_are_correct() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(11_016), "2000-02-29");
        assert_eq!(civil_date(20_748), "2026-10-22");
        assert_eq!(civil_date(20_817), "2026-12-30");
    }

    #[test]
    fn counts_and_enforces_the_limit() {
        let mut u = Usage::in_memory("2026-10-22");
        assert_eq!(u.acquire("g", Some(2), true).unwrap(), 1);
        assert_eq!(u.acquire("g", Some(2), true).unwrap(), 2);
        let err = u.acquire("g", Some(2), true).unwrap_err();
        assert_eq!((err.group.as_str(), err.limit), ("g", 2));
        assert_eq!(u.used("g"), 2, "a refused request is not counted");
        assert_eq!(u.acquire("other", Some(2), true).unwrap(), 1, "groups count separately");
    }

    #[test]
    fn unenforced_or_unlimited_still_counts() {
        let mut u = Usage::in_memory("2026-10-22");
        assert_eq!(u.acquire("g", Some(1), true).unwrap(), 1);
        assert_eq!(u.acquire("g", Some(1), false).unwrap(), 2);
        assert_eq!(u.acquire("free", None, true).unwrap(), 1);
        assert_eq!(u.acquire("free", None, true).unwrap(), 2);
    }

    #[test]
    fn a_new_day_starts_from_zero() {
        let path = temp_file("newday");
        Usage::load(path.clone(), "2026-10-22".into()).acquire("g", Some(5), true).unwrap();
        let mut tomorrow = Usage::load(path.clone(), "2026-10-23".into());
        assert_eq!(tomorrow.used("g"), 0);
        assert_eq!(tomorrow.acquire("g", Some(5), true).unwrap(), 1);
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(!saved.contains("2026-10-22"), "yesterday's counts are dropped: {saved}");
    }

    #[test]
    fn counts_survive_a_restart() {
        let path = temp_file("persist");
        let mut first = Usage::load(path.clone(), "2026-10-22".into());
        first.acquire("g", None, true).unwrap();
        first.acquire("g", None, true).unwrap();
        assert_eq!(Usage::load(path, "2026-10-22".into()).used("g"), 2);
    }

    #[test]
    fn a_corrupt_file_is_set_aside() {
        let path = temp_file("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();
        let u = Usage::load(path.clone(), "2026-10-22".into());
        assert_eq!(u.used("g"), 0);
        assert!(path.with_extension("json.corrupt").exists());
    }
}
