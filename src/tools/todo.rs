use serde_json::{json, Value};

/// In-memory task list for the agent to track progress on complex tasks.
/// Injected into the system prompt so the model never loses track of objectives.
#[derive(Debug, Clone)]
pub struct TodoList {
    items: Vec<TodoItem>,
    next_id: u32,
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub id: u32,
    pub text: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Done,
}

impl Default for TodoList {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoList {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Add a new todo item, returns its ID
    pub fn add(&mut self, text: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(TodoItem {
            id,
            text: text.to_string(),
            status: TodoStatus::Pending,
        });
        id
    }

    /// Update an item's status
    pub fn update(&mut self, id: u32, status: TodoStatus) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = status;
            true
        } else {
            false
        }
    }

    /// Remove an item by ID
    pub fn remove(&mut self, id: u32) -> bool {
        let len = self.items.len();
        self.items.retain(|i| i.id != id);
        self.items.len() < len
    }

    /// Clear all completed items
    pub fn clear_done(&mut self) {
        self.items.retain(|i| i.status != TodoStatus::Done);
    }

    /// Format the todo list for injection into the system prompt
    pub fn to_prompt_section(&self) -> String {
        if self.items.is_empty() {
            return String::new();
        }

        let mut s = String::from("\n## Current Task List\n");
        for item in &self.items {
            let marker = match item.status {
                TodoStatus::Pending => "[ ]",
                TodoStatus::InProgress => "[>]",
                TodoStatus::Done => "[x]",
            };
            s.push_str(&format!("{} #{}: {}\n", marker, item.id, item.text));
        }
        s.push_str("\nUpdate task status as you make progress. Mark tasks [x] when done.\n");
        s
    }

    /// One-line summary for status bar: shows current in-progress task or next pending
    pub fn current_task_summary(&self) -> Option<String> {
        // Show in-progress task first
        if let Some(item) = self
            .items
            .iter()
            .find(|i| i.status == TodoStatus::InProgress)
        {
            let done = self
                .items
                .iter()
                .filter(|i| i.status == TodoStatus::Done)
                .count();
            let total = self.items.len();
            return Some(format!("[{}/{}] {}", done, total, item.text));
        }
        // Fall back to next pending
        if let Some(item) = self.items.iter().find(|i| i.status == TodoStatus::Pending) {
            let done = self
                .items
                .iter()
                .filter(|i| i.status == TodoStatus::Done)
                .count();
            let total = self.items.len();
            return Some(format!("[{}/{}] {}", done, total, item.text));
        }
        // All done
        let total = self.items.len();
        if total > 0 {
            return Some(format!("[{}/{}] All tasks done!", total, total));
        }
        None
    }

    /// Format for display to the user
    pub fn to_display(&self) -> String {
        if self.items.is_empty() {
            return "No tasks. The agent will add tasks when working on complex requests."
                .to_string();
        }

        let mut s = String::new();
        for item in &self.items {
            let icon = match item.status {
                TodoStatus::Pending => "\u{2B1C}",     // white square
                TodoStatus::InProgress => "\u{1F7E8}", // yellow square
                TodoStatus::Done => "\u{2705}",        // check mark
            };
            s.push_str(&format!("{} #{}: {}\n", icon, item.id, item.text));
        }

        let pending = self
            .items
            .iter()
            .filter(|i| i.status == TodoStatus::Pending)
            .count();
        let in_progress = self
            .items
            .iter()
            .filter(|i| i.status == TodoStatus::InProgress)
            .count();
        let done = self
            .items
            .iter()
            .filter(|i| i.status == TodoStatus::Done)
            .count();

        s.push_str(&format!(
            "\n{} pending, {} in progress, {} done",
            pending, in_progress, done
        ));
        s
    }
}

/// Handle the todo_update tool call from the model
pub fn todo_update(args: &Value, todo: &mut TodoList) -> Value {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");

    match action {
        "add" => {
            let text = match args.get("text").and_then(|v| v.as_str()) {
                Some(t) => t,
                None => return json!({"error": "Missing 'text' for add action"}),
            };
            let id = todo.add(text);
            json!({"ok": true, "id": id, "action": "added"})
        }
        "start" => {
            let id = match args.get("id").and_then(|v| v.as_u64()) {
                Some(id) => id as u32,
                None => return json!({"error": "Missing 'id' for start action"}),
            };
            if todo.update(id, TodoStatus::InProgress) {
                json!({"ok": true, "id": id, "action": "started"})
            } else {
                json!({"error": format!("Task #{} not found", id)})
            }
        }
        "done" => {
            let id = match args.get("id").and_then(|v| v.as_u64()) {
                Some(id) => id as u32,
                None => return json!({"error": "Missing 'id' for done action"}),
            };
            if todo.update(id, TodoStatus::Done) {
                json!({"ok": true, "id": id, "action": "completed"})
            } else {
                json!({"error": format!("Task #{} not found", id)})
            }
        }
        "remove" => {
            let id = match args.get("id").and_then(|v| v.as_u64()) {
                Some(id) => id as u32,
                None => return json!({"error": "Missing 'id' for remove action"}),
            };
            if todo.remove(id) {
                json!({"ok": true, "id": id, "action": "removed"})
            } else {
                json!({"error": format!("Task #{} not found", id)})
            }
        }
        "list" => {
            json!({"tasks": todo.to_display()})
        }
        _ => {
            json!({"error": format!("Unknown action '{}'. Use: add, start, done, remove, list", action)})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_add_and_list() {
        let mut todo = TodoList::new();
        assert!(todo.is_empty());

        let id1 = todo.add("Write tests");
        let id2 = todo.add("Fix bug");
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert!(!todo.is_empty());
    }

    #[test]
    fn test_todo_status_update() {
        let mut todo = TodoList::new();
        let id = todo.add("Task 1");

        assert!(todo.update(id, TodoStatus::InProgress));
        assert_eq!(todo.items[0].status, TodoStatus::InProgress);

        assert!(todo.update(id, TodoStatus::Done));
        assert_eq!(todo.items[0].status, TodoStatus::Done);

        assert!(!todo.update(999, TodoStatus::Done)); // nonexistent
    }

    #[test]
    fn test_todo_remove() {
        let mut todo = TodoList::new();
        let id = todo.add("Task 1");
        todo.add("Task 2");

        assert!(todo.remove(id));
        assert_eq!(todo.items.len(), 1);
        assert!(!todo.remove(id)); // already removed
    }

    #[test]
    fn test_todo_clear_done() {
        let mut todo = TodoList::new();
        todo.add("Task 1");
        let id2 = todo.add("Task 2");
        todo.add("Task 3");

        todo.update(id2, TodoStatus::Done);
        todo.clear_done();
        assert_eq!(todo.items.len(), 2);
    }

    #[test]
    fn test_todo_prompt_section_empty() {
        let todo = TodoList::new();
        assert_eq!(todo.to_prompt_section(), "");
    }

    #[test]
    fn test_todo_prompt_section() {
        let mut todo = TodoList::new();
        todo.add("Write tests");
        let id = todo.add("Fix bug");
        todo.update(id, TodoStatus::Done);

        let section = todo.to_prompt_section();
        assert!(section.contains("[ ] #1: Write tests"));
        assert!(section.contains("[x] #2: Fix bug"));
        assert!(section.contains("Current Task List"));
    }

    #[test]
    fn test_todo_display() {
        let mut todo = TodoList::new();
        todo.add("Write tests");
        let id = todo.add("Fix bug");
        todo.update(id, TodoStatus::InProgress);

        let display = todo.to_display();
        assert!(display.contains("Write tests"));
        assert!(display.contains("Fix bug"));
        assert!(display.contains("1 pending"));
        assert!(display.contains("1 in progress"));
    }

    #[test]
    fn test_todo_update_tool_add() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "add", "text": "New task"}), &mut todo);
        assert!(result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert_eq!(todo.items.len(), 1);
    }

    #[test]
    fn test_todo_update_tool_done() {
        let mut todo = TodoList::new();
        let id = todo.add("Task");
        let result = todo_update(&json!({"action": "done", "id": id}), &mut todo);
        assert!(result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert_eq!(todo.items[0].status, TodoStatus::Done);
    }

    #[test]
    fn test_todo_update_tool_invalid() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "fly"}), &mut todo);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_todo_update_tool_missing_text() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "add"}), &mut todo);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_todo_update_tool_start() {
        let mut todo = TodoList::new();
        let id = todo.add("Task");
        let result = todo_update(&json!({"action": "start", "id": id}), &mut todo);
        assert!(result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert_eq!(todo.items[0].status, TodoStatus::InProgress);
    }

    #[test]
    fn test_todo_update_tool_start_missing_id() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "start"}), &mut todo);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_todo_update_tool_done_missing_id() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "done"}), &mut todo);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_todo_update_tool_remove() {
        let mut todo = TodoList::new();
        let id = todo.add("Task to remove");
        let result = todo_update(&json!({"action": "remove", "id": id}), &mut todo);
        assert!(result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(todo.is_empty());
    }

    #[test]
    fn test_todo_update_tool_remove_missing_id() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "remove"}), &mut todo);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_todo_update_tool_remove_nonexistent() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "remove", "id": 999}), &mut todo);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_todo_update_tool_list() {
        let mut todo = TodoList::new();
        todo.add("Task A");
        todo.add("Task B");
        let result = todo_update(&json!({"action": "list"}), &mut todo);
        let tasks = result.get("tasks").and_then(|v| v.as_str()).unwrap();
        assert!(tasks.contains("Task A"));
        assert!(tasks.contains("Task B"));
    }

    #[test]
    fn test_todo_display_empty() {
        let todo = TodoList::new();
        let display = todo.to_display();
        assert!(display.contains("No tasks"));
    }

    #[test]
    fn test_todo_update_tool_start_nonexistent() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "start", "id": 999}), &mut todo);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_todo_update_tool_done_nonexistent() {
        let mut todo = TodoList::new();
        let result = todo_update(&json!({"action": "done", "id": 999}), &mut todo);
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_todo_default() {
        let todo = TodoList::default();
        assert!(todo.is_empty());
    }

    #[test]
    fn test_current_task_summary_empty() {
        let todo = TodoList::new();
        assert!(todo.current_task_summary().is_none());
    }

    #[test]
    fn test_current_task_summary_pending() {
        let mut todo = TodoList::new();
        todo.add("Fix the bug");
        todo.add("Write tests");
        let summary = todo.current_task_summary().unwrap();
        assert!(summary.contains("[0/2]"));
        assert!(summary.contains("Fix the bug"));
    }

    #[test]
    fn test_current_task_summary_in_progress() {
        let mut todo = TodoList::new();
        let id = todo.add("Fix the bug");
        todo.add("Write tests");
        todo.update(id, TodoStatus::InProgress);
        let summary = todo.current_task_summary().unwrap();
        assert!(summary.contains("[0/2]"));
        assert!(summary.contains("Fix the bug"));
    }

    #[test]
    fn test_current_task_summary_partial_done() {
        let mut todo = TodoList::new();
        let id1 = todo.add("Fix the bug");
        todo.add("Write tests");
        todo.update(id1, TodoStatus::Done);
        let summary = todo.current_task_summary().unwrap();
        assert!(summary.contains("[1/2]"));
        assert!(summary.contains("Write tests"));
    }

    #[test]
    fn test_current_task_summary_all_done() {
        let mut todo = TodoList::new();
        let id1 = todo.add("Fix the bug");
        let id2 = todo.add("Write tests");
        todo.update(id1, TodoStatus::Done);
        todo.update(id2, TodoStatus::Done);
        let summary = todo.current_task_summary().unwrap();
        assert!(summary.contains("[2/2]"));
        assert!(summary.contains("All tasks done"));
    }
}
