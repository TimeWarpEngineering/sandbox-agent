//! Generated types from AI coding agent JSON schemas.
//!
//! This crate provides Rust types for:
//! - OpenCode SDK
//! - Claude Code SDK
//! - Codex SDK
//! - AMP Code SDK

pub mod opencode {
    //! OpenCode SDK types extracted from OpenAPI 3.1.1 spec.
    include!(concat!(env!("OUT_DIR"), "/opencode.rs"));
}

pub mod claude {
    //! Claude Code SDK types extracted from TypeScript definitions.
    include!(concat!(env!("OUT_DIR"), "/claude.rs"));
}

pub mod codex {
    //! Codex SDK types.
    include!(concat!(env!("OUT_DIR"), "/codex.rs"));
}

pub mod amp {
    //! AMP Code SDK types.
    include!(concat!(env!("OUT_DIR"), "/amp.rs"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_bash_input() {
        let input = claude::BashInput {
            command: "ls -la".to_string(),
            timeout: Some(5000.0),
            working_directory: None,
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("ls -la"));

        let parsed: claude::BashInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "ls -la");
    }

    #[test]
    fn test_codex_server_notification() {
        // Test ItemCompletedNotification with AgentMessage
        let notification = codex::ServerNotification::ItemCompleted(
            codex::ItemCompletedNotification {
                item: codex::ThreadItem::AgentMessage {
                    id: "msg-123".to_string(),
                    text: "Hello from Codex".to_string(),
                },
                thread_id: "thread-123".to_string(),
                turn_id: "turn-456".to_string(),
            }
        );

        let json = serde_json::to_string(&notification).unwrap();
        assert!(json.contains("item/completed"));
        assert!(json.contains("Hello from Codex"));
        assert!(json.contains("agentMessage"));
    }

    #[test]
    fn test_codex_thread_item_variants() {
        // Test UserMessage variant
        let user_msg = codex::ThreadItem::UserMessage {
            content: vec![codex::UserInput::Text {
                text: "Hello".to_string(),
                text_elements: vec![],
            }],
            id: "user-1".to_string(),
        };
        let json = serde_json::to_string(&user_msg).unwrap();
        assert!(json.contains("userMessage"));
        assert!(json.contains("Hello"));

        // Test CommandExecution variant
        let cmd = codex::ThreadItem::CommandExecution {
            aggregated_output: Some("output".to_string()),
            command: "ls -la".to_string(),
            command_actions: vec![],
            cwd: "/tmp".to_string(),
            duration_ms: Some(100),
            exit_code: Some(0),
            id: "cmd-1".to_string(),
            process_id: None,
            status: codex::CommandExecutionStatus::Completed,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("commandExecution"));
        assert!(json.contains("ls -la"));
    }

    #[test]
    fn test_amp_message() {
        let msg = amp::Message {
            role: amp::MessageRole::User,
            content: "Hello".to_string(),
            tool_calls: vec![],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }
}
