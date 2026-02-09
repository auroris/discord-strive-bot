use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Interaction {
    #[serde(rename = "type")]
    pub interaction_type: u8,
    pub token: String,
    pub id: String,
    pub application_id: String,
    pub data: Option<InteractionData>,
    pub member: Option<Member>,
    pub user: Option<User>,
    pub channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Member {
    pub user: Option<User>,
    pub nick: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: String,
    pub username: Option<String>,
    pub global_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionData {
    pub name: String,
    pub options: Option<Vec<CommandOption>>,
}

#[derive(Debug, Deserialize)]
pub struct CommandOption {
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct InteractionResponse {
    #[serde(rename = "type")]
    pub response_type: u8,
}

#[derive(Debug, Serialize)]
pub struct WebhookMessage {
    pub content: String,
}
