use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct ScoreStore {
    path: Option<PathBuf>,
}

impl ScoreStore {
    pub(crate) fn new() -> Self {
        Self { path: score_path() }
    }

    #[cfg(test)]
    fn at(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    pub(crate) fn load(&self) -> u32 {
        self.path
            .as_deref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0)
    }

    pub(crate) fn save(&self, score: u32) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, score.to_string())
    }
}

fn score_path() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("DONKEY_BAS_DATA_DIR") {
        return Some(Path::new(&directory).join("high-score"));
    }
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA")
            .map(|directory| Path::new(&directory).join("donkey-bas").join("high-score"));
    }
    std::env::var_os("HOME")
        .map(|directory| Path::new(&directory).join(".donkey-bas").join("high-score"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn high_score_round_trips() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("donkey-bas-score-{unique}"));
        let store = ScoreStore::at(path.clone());

        assert_eq!(store.load(), 0);
        store.save(42).unwrap();
        assert_eq!(store.load(), 42);

        let _ = fs::remove_file(path);
    }
}
