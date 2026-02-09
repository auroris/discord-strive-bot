use reqwest::blocking::Client;

use super::types::{ApiMessage, ClaudeRequest, ClaudeResponse, HistoryMessage};

pub fn call_claude_api(
    client: &Client,
    api_key: &str,
    request: &ClaudeRequest,
    debug: bool,
) -> Result<String, String> {
    let response_result = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "files-api-2025-04-14")
        .json(request)
        .send();

    match response_result {
        Ok(response) if response.status().is_success() => {
            match response.json::<ClaudeResponse>() {
                Ok(parsed) => parsed
                    .content
                    .first()
                    .map(|c| c.text.clone())
                    .ok_or_else(|| "No response content".to_string()),
                Err(e) => Err(format_error("Failed to parse Claude response", &e, request, debug)),
            }
        }
        Ok(response) => {
            let status = response.status();
            let body = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            Err(format_api_error(status, &body, request, debug))
        }
        Err(e) => Err(format_error("Failed to reach Claude API", &e, request, debug)),
    }
}

pub fn build_api_messages(
    history: &[HistoryMessage],
    mut context_blocks: Vec<serde_json::Value>,
) -> Vec<ApiMessage> {
    let mut api_messages = Vec::new();
    let mut first_user_seen = false;

    for msg in history {
        if msg.role == "user" && !first_user_seen && !context_blocks.is_empty() {
            context_blocks.push(serde_json::json!({
                "type": "text",
                "text": msg.content,
            }));
            api_messages.push(ApiMessage {
                role: "user".to_string(),
                content: serde_json::Value::Array(std::mem::take(&mut context_blocks)),
            });
            first_user_seen = true;
        } else {
            api_messages.push(ApiMessage {
                role: msg.role.clone(),
                content: serde_json::Value::String(msg.content.clone()),
            });
        }
    }

    api_messages
}

pub fn format_error(
    prefix: &str,
    error: &impl std::fmt::Display,
    request: &ClaudeRequest,
    debug: bool,
) -> String {
    if debug {
        let json = serde_json::to_string_pretty(request)
            .unwrap_or_else(|_| "Could not serialize request".to_string());
        format!("{}: {}\n\nRequest sent:\n{}", prefix, error, json)
    } else {
        format!("{}: {}", prefix, error)
    }
}

pub fn format_api_error(
    status: reqwest::StatusCode,
    body: &str,
    request: &ClaudeRequest,
    debug: bool,
) -> String {
    if debug {
        let json = serde_json::to_string_pretty(request)
            .unwrap_or_else(|_| "Could not serialize request".to_string());
        format!(
            "Claude API error ({}): {}\n\nRequest sent:\n{}",
            status, body, json
        )
    } else {
        format!("Claude API error ({}): {}", status, body)
    }
}
