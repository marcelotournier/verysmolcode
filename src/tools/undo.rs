use std::collections::HashMap;
use std::path::PathBuf;

/// A single file's state before it was changed.
#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub path: PathBuf,
    /// None if the file did not exist (was created by the agent).
    pub previous_content: Option<String>,
}

/// Tracks file changes from the most recent agent turn.
/// Only stores one turn's worth of snapshots to keep memory bounded.
#[derive(Debug, Default)]
pub struct UndoHistory {
    /// Snapshots from the current in-progress turn, keyed by path.
    /// HashMap ensures only one snapshot per file (the original state).
    pending: HashMap<PathBuf, Option<String>>,

    /// Snapshots from the last completed turn. This is what /undo reverts.
    last_turn: Option<Vec<FileSnapshot>>,
}

impl UndoHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Called at the start of each agent turn.
    pub fn begin_turn(&mut self) {
        self.pending.clear();
    }

    /// Called at the end of each agent turn.
    /// Moves pending snapshots to last_turn (only if changes were made).
    pub fn commit_turn(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        self.last_turn = Some(
            self.pending
                .drain()
                .map(|(path, content)| FileSnapshot {
                    path,
                    previous_content: content,
                })
                .collect(),
        );
    }

    /// Record the state of a file BEFORE it is mutated.
    /// Only records the first snapshot per path per turn.
    pub fn snapshot_before_write(&mut self, path: &std::path::Path) {
        let canonical = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };

        if self.pending.contains_key(&canonical) {
            return;
        }
        let content = std::fs::read_to_string(&canonical).ok();
        self.pending.insert(canonical, content);
    }

    /// Revert all files from the last completed turn.
    pub fn undo(&mut self) -> Result<Vec<String>, String> {
        let snapshots = self
            .last_turn
            .take()
            .ok_or_else(|| "Nothing to undo.".to_string())?;

        let mut reverted = Vec::new();
        for snapshot in &snapshots {
            match &snapshot.previous_content {
                Some(content) => {
                    std::fs::write(&snapshot.path, content).map_err(|e| {
                        format!("Failed to restore {}: {}", snapshot.path.display(), e)
                    })?;
                }
                None => {
                    if snapshot.path.exists() {
                        std::fs::remove_file(&snapshot.path).map_err(|e| {
                            format!("Failed to remove {}: {}", snapshot.path.display(), e)
                        })?;
                    }
                }
            }
            reverted.push(snapshot.path.display().to_string());
        }
        Ok(reverted)
    }

    pub fn has_undo(&self) -> bool {
        self.last_turn.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_undo_history_new() {
        let history = UndoHistory::new();
        assert!(!history.has_undo());
    }

    #[test]
    fn test_undo_nothing() {
        let mut history = UndoHistory::new();
        let result = history.undo();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Nothing to undo"));
    }

    #[test]
    fn test_begin_commit_empty_turn() {
        let mut history = UndoHistory::new();
        history.begin_turn();
        history.commit_turn();
        // No changes were made, so last_turn should remain None
        assert!(!history.has_undo());
    }

    #[test]
    fn test_snapshot_and_undo_existing_file() {
        let dir = std::env::temp_dir().join("vsc_undo_test_1");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("test.txt");
        fs::write(&file, "original content").unwrap();

        let mut history = UndoHistory::new();
        history.begin_turn();
        history.snapshot_before_write(&file);

        // Simulate file mutation
        fs::write(&file, "modified content").unwrap();

        history.commit_turn();
        assert!(history.has_undo());

        // Undo
        let reverted = history.undo().unwrap();
        assert_eq!(reverted.len(), 1);
        assert_eq!(fs::read_to_string(&file).unwrap(), "original content");

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_snapshot_and_undo_new_file() {
        let dir = std::env::temp_dir().join("vsc_undo_test_2");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("new_file.txt");

        // File doesn't exist yet
        assert!(!file.exists());

        let mut history = UndoHistory::new();
        history.begin_turn();
        history.snapshot_before_write(&file);

        // Simulate file creation
        fs::write(&file, "new content").unwrap();

        history.commit_turn();

        // Undo should delete the file
        let reverted = history.undo().unwrap();
        assert_eq!(reverted.len(), 1);
        assert!(!file.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_snapshot_dedup() {
        let dir = std::env::temp_dir().join("vsc_undo_test_3");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("dedup.txt");
        fs::write(&file, "v1").unwrap();

        let mut history = UndoHistory::new();
        history.begin_turn();
        history.snapshot_before_write(&file);

        // Modify file
        fs::write(&file, "v2").unwrap();

        // Snapshot again (should be ignored - already have v1)
        history.snapshot_before_write(&file);

        fs::write(&file, "v3").unwrap();

        history.commit_turn();

        // Undo should restore to v1 (original), not v2
        let _ = history.undo().unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "v1");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_undo_twice_fails() {
        let dir = std::env::temp_dir().join("vsc_undo_test_4");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("once.txt");
        fs::write(&file, "orig").unwrap();

        let mut history = UndoHistory::new();
        history.begin_turn();
        history.snapshot_before_write(&file);
        fs::write(&file, "changed").unwrap();
        history.commit_turn();

        assert!(history.undo().is_ok());
        assert!(!history.has_undo());
        assert!(history.undo().is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_new_turn_replaces_old() {
        let dir = std::env::temp_dir().join("vsc_undo_test_5");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("turns.txt");
        fs::write(&file, "v1").unwrap();

        let mut history = UndoHistory::new();

        // Turn 1
        history.begin_turn();
        history.snapshot_before_write(&file);
        fs::write(&file, "v2").unwrap();
        history.commit_turn();

        // Turn 2
        history.begin_turn();
        history.snapshot_before_write(&file);
        fs::write(&file, "v3").unwrap();
        history.commit_turn();

        // Undo should restore to v2 (state before turn 2), not v1
        let _ = history.undo().unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "v2");

        let _ = fs::remove_dir_all(&dir);
    }
}
