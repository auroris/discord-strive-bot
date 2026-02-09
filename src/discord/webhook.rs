use anyhow::Result;
use reqwest::blocking::Client;

use super::types::WebhookMessage;

pub fn send_to_discord(
    client: &Client,
    app_id: &str,
    token: &str,
    text: &str,
) -> Result<()> {
    let webhook_url = format!(
        "https://discord.com/api/v10/webhooks/{}/{}/messages/@original",
        app_id, token
    );

    let truncated = truncate_to_chars(text, 2000);

    client
        .patch(&webhook_url)
        .json(&WebhookMessage {
            content: truncated.to_string(),
        })
        .send()?;

    Ok(())
}

pub fn truncate_to_chars(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
