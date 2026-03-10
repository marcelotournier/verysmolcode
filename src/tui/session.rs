use crate::tui::app::DisplayMessage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_SESSIONS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    pub messages: Vec<SerializableMessage>,
    pub input_history: Vec<String>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_thinking_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializableMessage {
    User(String),
    Assistant(String),
    ToolCall(String),
    ToolResult(String),
    Status(String),
    Error(String),
    ModelInfo(String),
}

impl From<&DisplayMessage> for SerializableMessage {
    fn from(msg: &DisplayMessage) -> Self {
        match msg {
            DisplayMessage::User(s) => SerializableMessage::User(s.clone()),
            DisplayMessage::Assistant(s) => SerializableMessage::Assistant(s.clone()),
            DisplayMessage::ToolCall(s) => SerializableMessage::ToolCall(s.clone()),
            DisplayMessage::ToolResult(s) => SerializableMessage::ToolResult(s.clone()),
            DisplayMessage::Status(s) => SerializableMessage::Status(s.clone()),
            DisplayMessage::Error(s) => SerializableMessage::Error(s.clone()),
            DisplayMessage::ModelInfo(s) => SerializableMessage::ModelInfo(s.clone()),
        }
    }
}

impl From<&SerializableMessage> for DisplayMessage {
    fn from(msg: &SerializableMessage) -> Self {
        match msg {
            SerializableMessage::User(s) => DisplayMessage::User(s.clone()),
            SerializableMessage::Assistant(s) => DisplayMessage::Assistant(s.clone()),
            SerializableMessage::ToolCall(s) => DisplayMessage::ToolCall(s.clone()),
            SerializableMessage::ToolResult(s) => DisplayMessage::ToolResult(s.clone()),
            SerializableMessage::Status(s) => DisplayMessage::Status(s.clone()),
            SerializableMessage::Error(s) => DisplayMessage::Error(s.clone()),
            SerializableMessage::ModelInfo(s) => DisplayMessage::ModelInfo(s.clone()),
        }
    }
}

fn sessions_dir() -> PathBuf {
    crate::config::Config::config_dir().join("sessions")
}

impl Session {
    pub fn new(
        messages: &[DisplayMessage],
        input_history: &[String],
        total_input_tokens: u64,
        total_output_tokens: u64,
        total_thinking_tokens: u64,
    ) -> Self {
        let now = chrono::Local::now();
        Self {
            id: now.format("%Y%m%d-%H%M%S").to_string(),
            timestamp: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            messages: messages.iter().map(SerializableMessage::from).collect(),
            input_history: input_history.to_vec(),
            total_input_tokens,
            total_output_tokens,
            total_thinking_tokens,
        }
    }

    pub fn save(&self) -> Result<String, String> {
        let dir = sessions_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create sessions dir: {}", e))?;

        let path = dir.join(format!("{}.json", self.id));
        let data = serde_json::to_string(self).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(&path, data).map_err(|e| format!("Write error: {}", e))?;

        // Prune old sessions (keep only MAX_SESSIONS most recent)
        prune_sessions();

        Ok(path.display().to_string())
    }

    pub fn load_latest() -> Option<Session> {
        let dir = sessions_dir();
        if !dir.exists() {
            return None;
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();

        entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

        for entry in entries {
            if let Ok(data) = std::fs::read_to_string(entry.path()) {
                if let Ok(session) = serde_json::from_str::<Session>(&data) {
                    return Some(session);
                }
            }
        }
        None
    }

    pub fn list_recent(max: usize) -> Vec<(String, String, String, usize)> {
        let dir = sessions_dir();
        if !dir.exists() {
            return Vec::new();
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .ok()
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                    .collect()
            })
            .unwrap_or_default();

        entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
        entries.truncate(max);

        entries
            .into_iter()
            .filter_map(|entry| {
                let data = std::fs::read_to_string(entry.path()).ok()?;
                let session: Session = serde_json::from_str(&data).ok()?;
                let msg_count = session.messages.len();
                Some((session.id, session.timestamp, session.cwd, msg_count))
            })
            .collect()
    }

    pub fn load_by_id(id: &str) -> Option<Session> {
        let path = sessions_dir().join(format!("{}.json", id));
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn to_display_messages(&self) -> Vec<DisplayMessage> {
        self.messages.iter().map(DisplayMessage::from).collect()
    }
}

fn prune_sessions() {
    let dir = sessions_dir();
    if !dir.exists() {
        return;
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .ok()
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .collect()
        })
        .unwrap_or_default();

    if entries.len() <= MAX_SESSIONS {
        return;
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    // Remove oldest sessions beyond the limit
    for entry in entries.into_iter().skip(MAX_SESSIONS) {
        let _ = std::fs::remove_file(entry.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_serializable_message_roundtrip() {
        let display = DisplayMessage::User("hello".to_string());
        let serializable = SerializableMessage::from(&display);
        let back = DisplayMessage::from(&serializable);
        match back {
            DisplayMessage::User(s) => assert_eq!(s, "hello"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_serializable_message_all_variants() {
        let variants = vec![
            DisplayMessage::User("u".into()),
            DisplayMessage::Assistant("a".into()),
            DisplayMessage::ToolCall("tc".into()),
            DisplayMessage::ToolResult("tr".into()),
            DisplayMessage::Status("s".into()),
            DisplayMessage::Error("e".into()),
            DisplayMessage::ModelInfo("m".into()),
        ];
        for msg in &variants {
            let s = SerializableMessage::from(msg);
            let _back = DisplayMessage::from(&s);
        }
    }

    #[test]
    fn test_session_new() {
        let msgs = vec![DisplayMessage::User("test".into())];
        let history = vec!["test".to_string()];
        let session = Session::new(&msgs, &history, 100, 200, 50);
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.input_history.len(), 1);
        assert_eq!(session.total_input_tokens, 100);
        assert_eq!(session.total_output_tokens, 200);
        assert_eq!(session.total_thinking_tokens, 50);
        assert!(!session.id.is_empty());
    }

    #[test]
    fn test_session_save_and_load() {
        let temp = std::env::temp_dir().join("vsc-test-sessions");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let msgs = vec![
            DisplayMessage::User("hello".into()),
            DisplayMessage::Assistant("hi".into()),
        ];
        let session = Session {
            id: "test-session-001".to_string(),
            timestamp: "2024-01-01 00:00:00".to_string(),
            cwd: "/tmp".to_string(),
            messages: msgs.iter().map(SerializableMessage::from).collect(),
            input_history: vec!["hello".to_string()],
            total_input_tokens: 10,
            total_output_tokens: 20,
            total_thinking_tokens: 5,
        };

        let path = temp.join("test-session-001.json");
        let data = serde_json::to_string(&session).unwrap();
        fs::write(&path, &data).unwrap();

        let loaded: Session = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.id, "test-session-001");
        assert_eq!(loaded.messages.len(), 2);

        let display_msgs = loaded.to_display_messages();
        assert_eq!(display_msgs.len(), 2);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_session_list_empty() {
        // list_recent on non-existent dir returns empty
        let results = Session::list_recent(5);
        // May or may not be empty depending on prior test state, just check it doesn't panic
        let _ = results;
    }

    #[test]
    fn test_session_json_roundtrip() {
        let session = Session {
            id: "roundtrip".to_string(),
            timestamp: "2024-01-01".to_string(),
            cwd: "/home".to_string(),
            messages: vec![
                SerializableMessage::User("q".into()),
                SerializableMessage::Assistant("a".into()),
                SerializableMessage::ToolCall("read_file".into()),
                SerializableMessage::ToolResult("contents".into()),
                SerializableMessage::Status("ok".into()),
                SerializableMessage::Error("err".into()),
                SerializableMessage::ModelInfo("flash".into()),
            ],
            input_history: vec![],
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_thinking_tokens: 0,
        };
        let json = serde_json::to_string(&session).unwrap();
        let back: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(back.messages.len(), 7);
    }
}
