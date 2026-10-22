//! Past sessions: the question and everything that came back, kept in `history.json` in the state
//! directory so a long piece of research can be picked up again. A session is stored exactly as the
//! interface gives it (a JSON object); this module only needs its `id`, `prompt` and `askedAt`, plus a
//! few optional fields to summarise it, and keeps the newest first.

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use serde::Serialize;
use serde_json::{Value, json};

/// The most sessions kept; saving another drops the oldest.
pub const LIMIT: usize = 200;

/// A line in the list of sessions.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    pub id: String,
    /// When the question was asked, milliseconds since the Unix epoch.
    pub asked_at: i64,
    pub prompt: String,
    /// How many sources were asked, and how many of them answered.
    pub asked: usize,
    pub answered: usize,
    /// The source that wrote the merged reading, if there is one.
    pub merged_by: Option<String>,
}

/// A session needs an id, a question and a time; anything else is the interface's own.
pub fn check(session: &Value) -> anyhow::Result<()> {
    let id = session.get("id").and_then(Value::as_str).unwrap_or("");
    anyhow::ensure!(
        !id.is_empty() && id.len() <= 80 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "a session needs an id of letters, digits, - and _"
    );
    anyhow::ensure!(session.get("prompt").is_some_and(Value::is_string), "a session needs a prompt");
    anyhow::ensure!(session.get("askedAt").is_some_and(Value::is_i64), "a session needs the time it was asked (askedAt)");
    Ok(())
}

pub fn summarize(session: &Value) -> Option<Summary> {
    check(session).ok()?;
    let answers = session.get("answers").and_then(Value::as_object);
    let asked = session.get("asked").and_then(Value::as_array).map_or(0, Vec::len);
    let answered = session
        .get("asked")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .filter(|n| answers.and_then(|a| a.get(*n)).and_then(|a| a.get("ok")).and_then(Value::as_bool) == Some(true))
                .count()
        })
        .unwrap_or(0);
    Some(Summary {
        id: session["id"].as_str()?.to_string(),
        asked_at: session["askedAt"].as_i64()?,
        prompt: session["prompt"].as_str()?.to_string(),
        asked,
        answered,
        merged_by: session.get("merged").and_then(|m| m.get("provider")).and_then(Value::as_str).map(String::from),
    })
}

/// The history file.
pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every session, newest first. A missing file is no history; an unreadable one is moved to
    /// `history.json.corrupt` (so nothing is lost) and history starts again empty.
    pub fn load(&self) -> anyhow::Result<Vec<Value>> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", self.path.display())),
        };
        match serde_json::from_str::<Value>(&text).ok().and_then(|v| v.get("sessions").and_then(Value::as_array).cloned()) {
            Some(list) => Ok(list.into_iter().filter(|s| check(s).is_ok()).collect()),
            None => {
                let aside = self.path.with_extension("json.corrupt");
                let _ = std::fs::rename(&self.path, &aside);
                Ok(Vec::new())
            }
        }
    }

    fn write(&self, mut sessions: Vec<Value>) -> anyhow::Result<()> {
        sessions.sort_by_key(|s| std::cmp::Reverse(s["askedAt"].as_i64().unwrap_or(0)));
        sessions.truncate(LIMIT);
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let text = serde_json::to_string(&json!({"version": 1, "sessions": sessions}))? + "\n";
        // Write beside the file and rename, so a crash never leaves half of it.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).with_context(|| format!("replacing {}", self.path.display()))
    }

    pub fn summaries(&self) -> anyhow::Result<Vec<Summary>> {
        Ok(self.load()?.iter().filter_map(summarize).collect())
    }

    pub fn get(&self, id: &str) -> anyhow::Result<Value> {
        self.load()?.into_iter().find(|s| s["id"] == id).ok_or_else(|| anyhow!("that session is no longer in the history"))
    }

    /// Add a session, or replace the one with the same id.
    pub fn save(&self, session: Value) -> anyhow::Result<()> {
        check(&session)?;
        let mut sessions = self.load()?;
        sessions.retain(|s| s["id"] != session["id"]);
        sessions.push(session);
        self.write(sessions)
    }

    /// Returns whether the session was there.
    pub fn delete(&self, id: &str) -> anyhow::Result<bool> {
        let mut sessions = self.load()?;
        let before = sessions.len();
        sessions.retain(|s| s["id"] != id);
        let found = sessions.len() != before;
        if found {
            self.write(sessions)?;
        }
        Ok(found)
    }

    /// Delete everything; returns how many sessions there were.
    pub fn clear(&self) -> anyhow::Result<usize> {
        let n = self.load()?.len();
        if n > 0 {
            self.write(Vec::new())?;
        }
        Ok(n)
    }

    /// Add sessions from elsewhere (the interface's old local storage), skipping invalid ones and ones
    /// already here. Returns how many were added.
    pub fn import(&self, incoming: Vec<Value>) -> anyhow::Result<usize> {
        let mut sessions = self.load()?;
        let mut added = 0;
        for s in incoming {
            if check(&s).is_ok() && !sessions.iter().any(|have| have["id"] == s["id"]) {
                sessions.push(s);
                added += 1;
            }
        }
        if added > 0 {
            self.write(sessions)?;
        }
        Ok(added)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, asked_at: i64, prompt: &str) -> Value {
        json!({
            "id": id, "prompt": prompt, "askedAt": asked_at,
            "asked": ["a", "b", "c"],
            "answers": {"a": {"ok": true, "text": "x", "ms": 1}, "b": {"ok": false, "text": "HTTP 429", "ms": 2}, "c": {"ok": true, "text": "y", "ms": 3}},
            "merged": {"provider": "a", "text": "both"}
        })
    }

    fn store(tag: &str) -> (PathBuf, Store) {
        let dir = std::env::temp_dir().join(format!("pampas-history-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (dir.clone(), Store::new(dir.join("history.json")))
    }

    #[test]
    fn sessions_are_saved_listed_newest_first_replaced_and_deleted() {
        let (dir, s) = store("crud");
        assert!(s.summaries().unwrap().is_empty(), "no file is no history");
        s.save(session("one", 1000, "first?")).unwrap();
        s.save(session("two", 3000, "second?")).unwrap();
        s.save(session("three", 2000, "third?")).unwrap();
        let ids: Vec<String> = s.summaries().unwrap().into_iter().map(|x| x.id).collect();
        assert_eq!(ids, ["two", "three", "one"]);
        let first = &s.summaries().unwrap()[0];
        assert_eq!((first.asked, first.answered, first.merged_by.as_deref()), (3, 2, Some("a")));
        // saving the same id again replaces it
        s.save(session("one", 1000, "first, edited?")).unwrap();
        assert_eq!(s.summaries().unwrap().len(), 3);
        assert_eq!(s.get("one").unwrap()["prompt"], "first, edited?");
        assert!(s.get("nope").is_err());
        assert!(s.delete("three").unwrap());
        assert!(!s.delete("three").unwrap());
        assert_eq!(s.summaries().unwrap().len(), 2);
        assert_eq!(s.clear().unwrap(), 2);
        assert_eq!(s.clear().unwrap(), 0);
        assert!(s.summaries().unwrap().is_empty());
        assert!(!dir.join("history.json.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_sessions_are_refused_and_the_oldest_are_dropped_past_the_limit() {
        let (dir, s) = store("limit");
        assert!(s.save(json!({"prompt": "no id", "askedAt": 1})).is_err());
        assert!(s.save(json!({"id": "has space", "prompt": "q", "askedAt": 1})).is_err());
        assert!(s.save(json!({"id": "../escape", "prompt": "q", "askedAt": 1})).is_err());
        assert!(s.save(json!({"id": "x", "prompt": "q"})).is_err());
        let batch: Vec<Value> = (0..LIMIT as i64 + 5).map(|i| session(&format!("s{i}"), i, "q")).collect();
        assert_eq!(s.import(batch).unwrap(), LIMIT + 5);
        let kept = s.summaries().unwrap();
        assert_eq!(kept.len(), LIMIT);
        assert_eq!(kept[0].id, format!("s{}", LIMIT + 4), "newest kept");
        assert!(!kept.iter().any(|x| x.id == "s0"), "oldest dropped");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn import_skips_what_is_already_there_and_what_is_invalid() {
        let (dir, s) = store("import");
        s.save(session("have", 5, "q")).unwrap();
        let added = s.import(vec![session("have", 5, "q"), session("new", 6, "q"), json!({"nonsense": true})]).unwrap();
        assert_eq!(added, 1);
        assert_eq!(s.summaries().unwrap().len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_unreadable_file_is_moved_aside_not_lost() {
        let (dir, s) = store("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(s.path(), "{ not json").unwrap();
        assert!(s.summaries().unwrap().is_empty());
        assert_eq!(std::fs::read_to_string(dir.join("history.json.corrupt")).unwrap(), "{ not json");
        s.save(session("fresh", 1, "q")).unwrap();
        assert_eq!(s.summaries().unwrap().len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
