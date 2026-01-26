use crate::{
    AttachmentSource,
    ConversionError,
    CrashInfo,
    EventConversion,
    Started,
    UniversalEventData,
    UniversalMessage,
    UniversalMessageParsed,
    UniversalMessagePart,
};
use crate::codex as schema;
use serde_json::{Map, Value};

/// Convert a Codex ServerNotification to a universal event.
/// This is the main entry point for handling Codex events.
pub fn notification_to_universal(notification: &schema::ServerNotification) -> EventConversion {
    match notification {
        // Thread lifecycle
        schema::ServerNotification::ThreadStarted(params) => {
            thread_started_to_universal(params)
        }
        schema::ServerNotification::TurnStarted(params) => {
            turn_started_to_universal(params)
        }
        schema::ServerNotification::TurnCompleted(params) => {
            turn_completed_to_universal(params)
        }

        // Item lifecycle
        schema::ServerNotification::ItemStarted(params) => {
            item_started_to_universal(params)
        }
        schema::ServerNotification::ItemCompleted(params) => {
            item_completed_to_universal(params)
        }

        // Streaming deltas
        schema::ServerNotification::ItemAgentMessageDelta(params) => {
            agent_message_delta_to_universal(params)
        }
        schema::ServerNotification::ItemReasoningTextDelta(params) => {
            reasoning_text_delta_to_universal(params)
        }
        schema::ServerNotification::ItemReasoningSummaryTextDelta(params) => {
            reasoning_summary_delta_to_universal(params)
        }
        schema::ServerNotification::ItemCommandExecutionOutputDelta(params) => {
            command_output_delta_to_universal(params)
        }
        schema::ServerNotification::ItemFileChangeOutputDelta(params) => {
            file_change_delta_to_universal(params)
        }

        // Errors
        schema::ServerNotification::Error(params) => {
            error_notification_to_universal(params)
        }

        // Token usage updates
        schema::ServerNotification::ThreadTokenUsageUpdated(params) => {
            token_usage_to_universal(params)
        }

        // Turn diff updates
        schema::ServerNotification::TurnDiffUpdated(params) => {
            turn_diff_to_universal(params)
        }

        // Plan updates
        schema::ServerNotification::TurnPlanUpdated(params) => {
            turn_plan_to_universal(params)
        }

        // Terminal interaction
        schema::ServerNotification::ItemCommandExecutionTerminalInteraction(params) => {
            terminal_interaction_to_universal(params)
        }

        // MCP tool call progress
        schema::ServerNotification::ItemMcpToolCallProgress(params) => {
            mcp_progress_to_universal(params)
        }

        // Reasoning summary part added
        schema::ServerNotification::ItemReasoningSummaryPartAdded(params) => {
            reasoning_summary_part_to_universal(params)
        }

        // Context compacted
        schema::ServerNotification::ThreadCompacted(params) => {
            context_compacted_to_universal(params)
        }

        // Account/auth notifications (less relevant for message conversion)
        schema::ServerNotification::AccountUpdated(_)
        | schema::ServerNotification::AccountRateLimitsUpdated(_)
        | schema::ServerNotification::AccountLoginCompleted(_)
        | schema::ServerNotification::McpServerOauthLoginCompleted(_)
        | schema::ServerNotification::AuthStatusChange(_)
        | schema::ServerNotification::LoginChatGptComplete(_)
        | schema::ServerNotification::SessionConfigured(_)
        | schema::ServerNotification::DeprecationNotice(_)
        | schema::ServerNotification::ConfigWarning(_)
        | schema::ServerNotification::WindowsWorldWritableWarning(_)
        | schema::ServerNotification::RawResponseItemCompleted(_) => {
            EventConversion::new(UniversalEventData::Unknown {
                raw: serde_json::to_value(notification).unwrap_or(Value::Null),
            })
        }
    }
}

fn thread_started_to_universal(params: &schema::ThreadStartedNotification) -> EventConversion {
    let started = Started {
        message: Some("thread/started".to_string()),
        details: serde_json::to_value(&params.thread).ok(),
    };
    EventConversion::new(UniversalEventData::Started { started })
        .with_session(Some(params.thread.id.clone()))
}

fn turn_started_to_universal(params: &schema::TurnStartedNotification) -> EventConversion {
    let started = Started {
        message: Some("turn/started".to_string()),
        details: serde_json::to_value(&params.turn).ok(),
    };
    EventConversion::new(UniversalEventData::Started { started })
        .with_session(Some(params.thread_id.clone()))
}

fn turn_completed_to_universal(params: &schema::TurnCompletedNotification) -> EventConversion {
    // Convert all items in the turn to messages
    let items = &params.turn.items;
    if items.is_empty() {
        // If no items, just emit as unknown with the turn data
        return EventConversion::new(UniversalEventData::Unknown {
            raw: serde_json::to_value(params).unwrap_or(Value::Null),
        })
        .with_session(Some(params.thread_id.clone()));
    }

    // Return the last item as a message (most relevant for completion)
    if let Some(last_item) = items.last() {
        let message = thread_item_to_message(last_item);
        EventConversion::new(UniversalEventData::Message { message })
            .with_session(Some(params.thread_id.clone()))
    } else {
        EventConversion::new(UniversalEventData::Unknown {
            raw: serde_json::to_value(params).unwrap_or(Value::Null),
        })
        .with_session(Some(params.thread_id.clone()))
    }
}

fn item_started_to_universal(params: &schema::ItemStartedNotification) -> EventConversion {
    let message = thread_item_to_message(&params.item);
    EventConversion::new(UniversalEventData::Message { message })
        .with_session(Some(params.thread_id.clone()))
}

fn item_completed_to_universal(params: &schema::ItemCompletedNotification) -> EventConversion {
    let message = thread_item_to_message(&params.item);
    EventConversion::new(UniversalEventData::Message { message })
        .with_session(Some(params.thread_id.clone()))
}

fn agent_message_delta_to_universal(
    params: &schema::AgentMessageDeltaNotification,
) -> EventConversion {
    let message = UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(params.item_id.clone()),
        metadata: Map::from_iter([
            ("delta".to_string(), Value::Bool(true)),
            ("turnId".to_string(), Value::String(params.turn_id.clone())),
        ]),
        parts: vec![UniversalMessagePart::Text {
            text: params.delta.clone(),
        }],
    });
    EventConversion::new(UniversalEventData::Message { message })
        .with_session(Some(params.thread_id.clone()))
}

fn reasoning_text_delta_to_universal(
    params: &schema::ReasoningTextDeltaNotification,
) -> EventConversion {
    let message = UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(params.item_id.clone()),
        metadata: Map::from_iter([
            ("delta".to_string(), Value::Bool(true)),
            ("itemType".to_string(), Value::String("reasoning".to_string())),
            ("turnId".to_string(), Value::String(params.turn_id.clone())),
        ]),
        parts: vec![UniversalMessagePart::Text {
            text: params.delta.clone(),
        }],
    });
    EventConversion::new(UniversalEventData::Message { message })
        .with_session(Some(params.thread_id.clone()))
}

fn reasoning_summary_delta_to_universal(
    params: &schema::ReasoningSummaryTextDeltaNotification,
) -> EventConversion {
    let message = UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(params.item_id.clone()),
        metadata: Map::from_iter([
            ("delta".to_string(), Value::Bool(true)),
            ("itemType".to_string(), Value::String("reasoning_summary".to_string())),
            ("turnId".to_string(), Value::String(params.turn_id.clone())),
        ]),
        parts: vec![UniversalMessagePart::Text {
            text: params.delta.clone(),
        }],
    });
    EventConversion::new(UniversalEventData::Message { message })
        .with_session(Some(params.thread_id.clone()))
}

fn command_output_delta_to_universal(
    params: &schema::CommandExecutionOutputDeltaNotification,
) -> EventConversion {
    let message = UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(params.item_id.clone()),
        metadata: Map::from_iter([
            ("delta".to_string(), Value::Bool(true)),
            ("itemType".to_string(), Value::String("commandExecution".to_string())),
            ("turnId".to_string(), Value::String(params.turn_id.clone())),
        ]),
        parts: vec![UniversalMessagePart::Text {
            text: params.delta.clone(),
        }],
    });
    EventConversion::new(UniversalEventData::Message { message })
        .with_session(Some(params.thread_id.clone()))
}

fn file_change_delta_to_universal(
    params: &schema::FileChangeOutputDeltaNotification,
) -> EventConversion {
    let message = UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(params.item_id.clone()),
        metadata: Map::from_iter([
            ("delta".to_string(), Value::Bool(true)),
            ("itemType".to_string(), Value::String("fileChange".to_string())),
            ("turnId".to_string(), Value::String(params.turn_id.clone())),
        ]),
        parts: vec![UniversalMessagePart::Text {
            text: params.delta.clone(),
        }],
    });
    EventConversion::new(UniversalEventData::Message { message })
        .with_session(Some(params.thread_id.clone()))
}

fn error_notification_to_universal(params: &schema::ErrorNotification) -> EventConversion {
    let crash = CrashInfo {
        message: params.error.message.clone(),
        kind: Some("error".to_string()),
        details: serde_json::to_value(params).ok(),
    };
    EventConversion::new(UniversalEventData::Error { error: crash })
        .with_session(Some(params.thread_id.clone()))
}

fn token_usage_to_universal(
    params: &schema::ThreadTokenUsageUpdatedNotification,
) -> EventConversion {
    EventConversion::new(UniversalEventData::Unknown {
        raw: serde_json::to_value(params).unwrap_or(Value::Null),
    })
    .with_session(Some(params.thread_id.clone()))
}

fn turn_diff_to_universal(params: &schema::TurnDiffUpdatedNotification) -> EventConversion {
    EventConversion::new(UniversalEventData::Unknown {
        raw: serde_json::to_value(params).unwrap_or(Value::Null),
    })
    .with_session(Some(params.thread_id.clone()))
}

fn turn_plan_to_universal(params: &schema::TurnPlanUpdatedNotification) -> EventConversion {
    EventConversion::new(UniversalEventData::Unknown {
        raw: serde_json::to_value(params).unwrap_or(Value::Null),
    })
    .with_session(Some(params.thread_id.clone()))
}

fn terminal_interaction_to_universal(
    params: &schema::TerminalInteractionNotification,
) -> EventConversion {
    EventConversion::new(UniversalEventData::Unknown {
        raw: serde_json::to_value(params).unwrap_or(Value::Null),
    })
    .with_session(Some(params.thread_id.clone()))
}

fn mcp_progress_to_universal(params: &schema::McpToolCallProgressNotification) -> EventConversion {
    EventConversion::new(UniversalEventData::Unknown {
        raw: serde_json::to_value(params).unwrap_or(Value::Null),
    })
    .with_session(Some(params.thread_id.clone()))
}

fn reasoning_summary_part_to_universal(
    params: &schema::ReasoningSummaryPartAddedNotification,
) -> EventConversion {
    // This notification signals a new summary part was added, but doesn't contain the text itself
    // Return as Unknown with all metadata
    EventConversion::new(UniversalEventData::Unknown {
        raw: serde_json::to_value(params).unwrap_or(Value::Null),
    })
    .with_session(Some(params.thread_id.clone()))
}

fn context_compacted_to_universal(
    params: &schema::ContextCompactedNotification,
) -> EventConversion {
    EventConversion::new(UniversalEventData::Unknown {
        raw: serde_json::to_value(params).unwrap_or(Value::Null),
    })
    .with_session(Some(params.thread_id.clone()))
}

/// Convert a ThreadItem to a UniversalMessage
pub fn thread_item_to_message(item: &schema::ThreadItem) -> UniversalMessage {
    match item {
        schema::ThreadItem::UserMessage { content, id } => {
            user_message_to_universal(content, id)
        }
        schema::ThreadItem::AgentMessage { id, text } => {
            agent_message_to_universal(id, text)
        }
        schema::ThreadItem::Reasoning { content, id, summary } => {
            reasoning_to_universal(id, content, summary)
        }
        schema::ThreadItem::CommandExecution {
            aggregated_output,
            command,
            command_actions: _,
            cwd,
            duration_ms,
            exit_code,
            id,
            process_id: _,
            status,
        } => {
            command_execution_to_universal(
                id,
                command,
                cwd,
                aggregated_output.as_deref(),
                exit_code.as_ref(),
                duration_ms.as_ref(),
                status,
            )
        }
        schema::ThreadItem::FileChange { changes, id, status } => {
            file_change_to_universal(id, changes, status)
        }
        schema::ThreadItem::McpToolCall {
            arguments,
            duration_ms: _,
            error,
            id,
            result,
            server,
            status,
            tool,
        } => {
            mcp_tool_call_to_universal(id, server, tool, arguments, result.as_ref(), error.as_ref(), status)
        }
        schema::ThreadItem::CollabAgentToolCall {
            agents_states: _,
            id,
            prompt,
            receiver_thread_ids: _,
            sender_thread_id,
            status,
            tool,
        } => {
            collab_tool_call_to_universal(id, tool, prompt.as_deref(), sender_thread_id, status)
        }
        schema::ThreadItem::WebSearch { id, query } => {
            web_search_to_universal(id, query)
        }
        schema::ThreadItem::ImageView { id, path } => {
            image_view_to_universal(id, path)
        }
        schema::ThreadItem::EnteredReviewMode { id, review } => {
            review_mode_to_universal(id, review, true)
        }
        schema::ThreadItem::ExitedReviewMode { id, review } => {
            review_mode_to_universal(id, review, false)
        }
    }
}

fn user_message_to_universal(content: &[schema::UserInput], id: &str) -> UniversalMessage {
    let parts: Vec<UniversalMessagePart> = content.iter().map(user_input_to_part).collect();
    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "user".to_string(),
        id: Some(id.to_string()),
        metadata: Map::new(),
        parts,
    })
}

fn user_input_to_part(input: &schema::UserInput) -> UniversalMessagePart {
    match input {
        schema::UserInput::Text { text, text_elements: _ } => {
            UniversalMessagePart::Text { text: text.clone() }
        }
        schema::UserInput::Image { image_url } => {
            UniversalMessagePart::Image {
                source: AttachmentSource::Url { url: image_url.clone() },
                mime_type: None,
                alt: None,
                raw: None,
            }
        }
        schema::UserInput::LocalImage { path } => {
            UniversalMessagePart::Image {
                source: AttachmentSource::Path { path: path.clone() },
                mime_type: None,
                alt: None,
                raw: None,
            }
        }
        schema::UserInput::Skill { name, path } => {
            UniversalMessagePart::Unknown {
                raw: serde_json::json!({
                    "type": "skill",
                    "name": name,
                    "path": path,
                }),
            }
        }
    }
}

fn agent_message_to_universal(id: &str, text: &str) -> UniversalMessage {
    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata: Map::from_iter([
            ("itemType".to_string(), Value::String("agentMessage".to_string())),
        ]),
        parts: vec![UniversalMessagePart::Text {
            text: text.to_string(),
        }],
    })
}

fn reasoning_to_universal(
    id: &str,
    content: &[String],
    summary: &[String],
) -> UniversalMessage {
    let mut metadata = Map::new();
    metadata.insert("itemType".to_string(), Value::String("reasoning".to_string()));
    if !summary.is_empty() {
        metadata.insert(
            "summary".to_string(),
            Value::Array(summary.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }

    let parts: Vec<UniversalMessagePart> = content
        .iter()
        .map(|text| UniversalMessagePart::Text { text: text.clone() })
        .collect();

    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata,
        parts,
    })
}

fn command_execution_to_universal(
    id: &str,
    command: &str,
    cwd: &str,
    aggregated_output: Option<&str>,
    exit_code: Option<&i32>,
    duration_ms: Option<&i64>,
    status: &schema::CommandExecutionStatus,
) -> UniversalMessage {
    let mut metadata = Map::new();
    metadata.insert("itemType".to_string(), Value::String("commandExecution".to_string()));
    metadata.insert("command".to_string(), Value::String(command.to_string()));
    metadata.insert("cwd".to_string(), Value::String(cwd.to_string()));
    metadata.insert("status".to_string(), Value::String(format!("{:?}", status)));
    if let Some(code) = exit_code {
        metadata.insert("exitCode".to_string(), Value::Number((*code).into()));
    }
    if let Some(ms) = duration_ms {
        metadata.insert("durationMs".to_string(), Value::Number((*ms).into()));
    }

    let parts = if let Some(output) = aggregated_output {
        vec![UniversalMessagePart::Text {
            text: output.to_string(),
        }]
    } else {
        vec![]
    };

    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata,
        parts,
    })
}

fn file_change_to_universal(
    id: &str,
    changes: &[schema::FileUpdateChange],
    status: &schema::PatchApplyStatus,
) -> UniversalMessage {
    let mut metadata = Map::new();
    metadata.insert("itemType".to_string(), Value::String("fileChange".to_string()));
    metadata.insert("status".to_string(), Value::String(format!("{:?}", status)));

    let parts: Vec<UniversalMessagePart> = changes
        .iter()
        .map(|change| {
            let raw = serde_json::to_value(change).unwrap_or(Value::Null);
            UniversalMessagePart::Unknown { raw }
        })
        .collect();

    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata,
        parts,
    })
}

fn mcp_tool_call_to_universal(
    id: &str,
    server: &str,
    tool: &str,
    arguments: &Value,
    result: Option<&schema::McpToolCallResult>,
    error: Option<&schema::McpToolCallError>,
    status: &schema::McpToolCallStatus,
) -> UniversalMessage {
    let mut metadata = Map::new();
    metadata.insert("itemType".to_string(), Value::String("mcpToolCall".to_string()));
    metadata.insert("server".to_string(), Value::String(server.to_string()));
    metadata.insert("status".to_string(), Value::String(format!("{:?}", status)));

    let is_error = error.is_some();
    let result_value = if let Some(res) = result {
        serde_json::to_value(res).unwrap_or(Value::Null)
    } else if let Some(err) = error {
        serde_json::to_value(err).unwrap_or(Value::Null)
    } else {
        Value::Null
    };

    let parts = vec![
        UniversalMessagePart::ToolCall {
            id: Some(id.to_string()),
            name: tool.to_string(),
            input: arguments.clone(),
        },
        UniversalMessagePart::ToolResult {
            id: Some(id.to_string()),
            name: Some(tool.to_string()),
            output: result_value,
            is_error: Some(is_error),
        },
    ];

    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata,
        parts,
    })
}

fn collab_tool_call_to_universal(
    id: &str,
    tool: &schema::CollabAgentTool,
    prompt: Option<&str>,
    sender_thread_id: &str,
    status: &schema::CollabAgentToolCallStatus,
) -> UniversalMessage {
    let mut metadata = Map::new();
    metadata.insert("itemType".to_string(), Value::String("collabAgentToolCall".to_string()));
    metadata.insert("tool".to_string(), Value::String(format!("{:?}", tool)));
    metadata.insert("senderThreadId".to_string(), Value::String(sender_thread_id.to_string()));
    metadata.insert("status".to_string(), Value::String(format!("{:?}", status)));

    let parts = if let Some(p) = prompt {
        vec![UniversalMessagePart::Text { text: p.to_string() }]
    } else {
        vec![]
    };

    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata,
        parts,
    })
}

fn web_search_to_universal(id: &str, query: &str) -> UniversalMessage {
    let mut metadata = Map::new();
    metadata.insert("itemType".to_string(), Value::String("webSearch".to_string()));

    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata,
        parts: vec![UniversalMessagePart::Text {
            text: query.to_string(),
        }],
    })
}

fn image_view_to_universal(id: &str, path: &str) -> UniversalMessage {
    let mut metadata = Map::new();
    metadata.insert("itemType".to_string(), Value::String("imageView".to_string()));

    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata,
        parts: vec![UniversalMessagePart::Image {
            source: AttachmentSource::Path { path: path.to_string() },
            mime_type: None,
            alt: None,
            raw: None,
        }],
    })
}

fn review_mode_to_universal(id: &str, review: &str, entered: bool) -> UniversalMessage {
    let item_type = if entered { "enteredReviewMode" } else { "exitedReviewMode" };
    let mut metadata = Map::new();
    metadata.insert("itemType".to_string(), Value::String(item_type.to_string()));
    metadata.insert("review".to_string(), Value::String(review.to_string()));

    UniversalMessage::Parsed(UniversalMessageParsed {
        role: "assistant".to_string(),
        id: Some(id.to_string()),
        metadata,
        parts: vec![],
    })
}

/// Convert a universal event back to a Codex ServerNotification.
/// Note: This is a best-effort conversion and may not preserve all information.
pub fn universal_event_to_codex(
    event: &UniversalEventData,
) -> Result<schema::ServerNotification, ConversionError> {
    match event {
        UniversalEventData::Message { message } => {
            let parsed = match message {
                UniversalMessage::Parsed(parsed) => parsed,
                UniversalMessage::Unparsed { .. } => {
                    return Err(ConversionError::Unsupported("unparsed message"))
                }
            };

            // Extract text content
            let text = parsed
                .parts
                .iter()
                .filter_map(|part| {
                    if let UniversalMessagePart::Text { text } = part {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            let id = parsed.id.clone().unwrap_or_else(|| "msg".to_string());
            let thread_id = "unknown".to_string();
            let turn_id = "unknown".to_string();

            // Create an ItemCompletedNotification with an AgentMessage item
            let item = schema::ThreadItem::AgentMessage {
                id,
                text,
            };

            Ok(schema::ServerNotification::ItemCompleted(
                schema::ItemCompletedNotification {
                    item,
                    thread_id,
                    turn_id,
                },
            ))
        }
        UniversalEventData::Error { error } => {
            let turn_error = schema::TurnError {
                message: error.message.clone(),
                additional_details: error.details.as_ref().and_then(|d| {
                    d.get("additionalDetails")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                }),
                codex_error_info: None,
            };

            Ok(schema::ServerNotification::Error(schema::ErrorNotification {
                error: turn_error,
                thread_id: "unknown".to_string(),
                turn_id: "unknown".to_string(),
                will_retry: false,
            }))
        }
        _ => Err(ConversionError::Unsupported("codex event type")),
    }
}
