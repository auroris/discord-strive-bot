use anyhow::Result;
use discord_strive::claude::api::{build_api_messages, call_claude_api};
use discord_strive::claude::context::{load_context_file_blocks, load_system_extension};
use discord_strive::claude::history::{
    get_history_path, load_history, save_history, MAX_HISTORY_MESSAGES,
};
use discord_strive::claude::types::{ClaudeRequest, HistoryMessage};
use discord_strive::discord::webhook::send_to_discord;
use reqwest::blocking::Client;
use std::env;

fn prepend_errors(errors: &[String], text: &str) -> String {
    if errors.is_empty() {
        text.to_string()
    } else {
        format!("{}\n\n{}", errors.join("\n"), text)
    }
}

fn is_debug_mode() -> bool {
    env::var("STRIVE_DEBUG").map_or(false, |v| v == "1" || v == "true")
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 6 {
        eprintln!(
            "Usage: callback-handler <app_id> <token> <user_id> <nickname> <channel_id> [text]"
        );
        std::process::exit(1);
    }

    let app_id = &args[1];
    let token = &args[2];
    let user_id = &args[3];
    let nickname = &args[4];
    let channel_id = &args[5];
    let user_text = args.get(6).map(|s| s.as_str()).unwrap_or("");

    let api_key = env::var("ANTHROPIC_API_KEY")?;
    let debug = is_debug_mode();

    // System prompt with optional extension
    let (system_extension, system_errors) = load_system_extension();
    let system_prompt = if system_extension.is_empty() {
        format!(
            "You are Strive, a helpful AI assistant of the Morlencir Empire. The user is known as {}.",
            nickname
        )
    } else {
        format!(
            "You are Strive, a helpful AI assistant of the Morlencir Empire. The user is known as {}.\n\n{}",
            nickname, system_extension
        )
    };

    let history_path = get_history_path(user_id, channel_id);
    let mut history = load_history(&history_path)?;
    history.push(HistoryMessage {
        role: "user".to_string(),
        content: user_text.to_string(),
    });

    let (context_blocks, context_errors) = load_context_file_blocks();
    let mut all_errors = system_errors;
    all_errors.extend(context_errors);
    let api_messages = build_api_messages(&history, context_blocks);

    let request = ClaudeRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        max_tokens: 1024,
        system: system_prompt,
        messages: api_messages,
    };

    let client = Client::new();

    match call_claude_api(&client, &api_key, &request, debug) {
        Ok(text) => {
            history.push(HistoryMessage {
                role: "assistant".to_string(),
                content: text.clone(),
            });
            if history.len() > MAX_HISTORY_MESSAGES {
                history = history[history.len() - MAX_HISTORY_MESSAGES..].to_vec();
            }
            save_history(&history_path, &history)?;
            send_to_discord(&client, app_id, token, &prepend_errors(&all_errors, &text))?;
        }
        Err(error_text) => {
            send_to_discord(&client, app_id, token, &prepend_errors(&all_errors, &error_text))?;
        }
    }

    Ok(())
}
