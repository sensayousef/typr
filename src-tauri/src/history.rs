use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const MAX_ENTRIES: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub text: String,
    pub timestamp: u64,
    pub engine: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct History {
    entries: Vec<HistoryEntry>,
}

impl History {
    fn path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("history.json")
    }

    pub fn load(app_dir: &PathBuf) -> Self {
        let path = Self::path(app_dir);
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn push(&mut self, entry: HistoryEntry, app_dir: &PathBuf) {
        self.entries.insert(0, entry);
        if self.entries.len() > MAX_ENTRIES {
            self.entries.truncate(MAX_ENTRIES);
        }
        let _ = self.save(app_dir);
    }

    pub fn clear(&mut self, app_dir: &PathBuf) {
        self.entries.clear();
        let _ = self.save(app_dir);
    }

    pub fn list(&self) -> &[HistoryEntry] {
        &self.entries
    }

    fn save(&self, app_dir: &PathBuf) -> Result<(), String> {
        let path = Self::path(app_dir);
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    fn make_entry(text: &str) -> HistoryEntry {
        HistoryEntry {
            text: text.to_string(),
            timestamp: 0,
            engine: "local".to_string(),
        }
    }

    #[test]
    fn push_prepends_newest_first() {
        let dir = temp_dir().join("robin_test_history");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut h = History::default();
        h.push(make_entry("first"), &dir);
        h.push(make_entry("second"), &dir);

        assert_eq!(h.list()[0].text, "second");
        assert_eq!(h.list()[1].text, "first");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn caps_at_max_entries() {
        let dir = temp_dir().join("robin_test_history_cap");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut h = History::default();
        for i in 0..=MAX_ENTRIES + 5 {
            h.push(make_entry(&format!("entry {i}")), &dir);
        }
        assert_eq!(h.list().len(), MAX_ENTRIES);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_empties_list() {
        let dir = temp_dir().join("robin_test_history_clear");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut h = History::default();
        h.push(make_entry("something"), &dir);
        h.clear(&dir);
        assert!(h.list().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
