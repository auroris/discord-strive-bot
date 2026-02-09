use anyhow::{anyhow, Result};
use discord_strive::discord::types::{Interaction, InteractionResponse};
use discord_strive::discord::verify::verify_signature;
use fork::{daemon, Fork};
use std::env;
use std::io::{self, Read, Write};
use std::process::Command;

fn main() -> Result<()> {
    let public_key = env::var("DISCORD_PUBLIC_KEY")?;
    let callback_binary = env::var("CALLBACK_BINARY_PATH")
        .unwrap_or_else(|_| "./target/release/callback-handler".to_string());

    let signature = env::var("HTTP_X_SIGNATURE_ED25519").unwrap_or_default();
    let timestamp = env::var("HTTP_X_SIGNATURE_TIMESTAMP").unwrap_or_default();

    let mut body = String::new();
    io::stdin().read_to_string(&mut body)?;

    if signature.is_empty() || timestamp.is_empty() {
        print!("Content-Type: text/plain\r\n\r\n");
        print!("Strive: Hello, there!");
        return Ok(());
    }

    verify_signature(&public_key, &signature, &timestamp, &body)?;

    let interaction: Interaction = serde_json::from_str(&body)?;

    match interaction.interaction_type {
        1 => send_cgi_json(&InteractionResponse { response_type: 1 }),
        2 => handle_command(&interaction, &callback_binary),
        other => Err(anyhow!("Unhandled interaction type: {other}")),
    }
}

fn send_cgi_json(response: &InteractionResponse) -> Result<()> {
    let json = serde_json::to_string(response)?;
    print!("Status: 200 OK\r\n");
    print!("Content-Type: application/json\r\n");
    print!("Content-Length: {}\r\n", json.len());
    print!("\r\n");
    print!("{json}");
    io::stdout().flush()?;
    Ok(())
}

fn handle_command(interaction: &Interaction, callback_binary: &str) -> Result<()> {
    let data = interaction
        .data
        .as_ref()
        .ok_or_else(|| anyhow!("Missing interaction data"))?;

    if data.name != "strive" {
        return Err(anyhow!("Unknown command: {}", data.name));
    }

    send_cgi_json(&InteractionResponse { response_type: 5 })?;

    let text = data
        .options
        .as_ref()
        .and_then(|opts| opts.iter().find(|o| o.name == "text"))
        .map(|o| o.value.as_str().unwrap_or(""))
        .unwrap_or("");

    let user_id = resolve_user_id(interaction);
    let display_name = resolve_display_name(interaction);
    let channel_id = interaction.channel_id.as_deref().unwrap_or("unknown");

    spawn_callback(
        callback_binary,
        &interaction.application_id,
        &interaction.token,
        user_id,
        display_name,
        channel_id,
        text,
    );

    Ok(())
}

fn resolve_user_id(interaction: &Interaction) -> &str {
    interaction
        .member
        .as_ref()
        .and_then(|m| m.user.as_ref())
        .or(interaction.user.as_ref())
        .map(|u| u.id.as_str())
        .unwrap_or("unknown")
}

fn resolve_display_name(interaction: &Interaction) -> &str {
    let member_nick = interaction
        .member
        .as_ref()
        .and_then(|m| m.nick.as_deref());

    let member_user = interaction.member.as_ref().and_then(|m| m.user.as_ref());
    let dm_user = interaction.user.as_ref();

    member_nick
        .or_else(|| member_user.and_then(|u| u.global_name.as_deref()))
        .or_else(|| member_user.and_then(|u| u.username.as_deref()))
        .or_else(|| dm_user.and_then(|u| u.global_name.as_deref()))
        .or_else(|| dm_user.and_then(|u| u.username.as_deref()))
        .unwrap_or("someone")
}

fn spawn_callback(
    binary: &str,
    app_id: &str,
    token: &str,
    user_id: &str,
    display_name: &str,
    channel_id: &str,
    text: &str,
) {
    if let Ok(Fork::Child) = daemon(false, false) {
        let _ = Command::new(binary)
            .args([app_id, token, user_id, display_name, channel_id, text])
            .output();
    }
}
