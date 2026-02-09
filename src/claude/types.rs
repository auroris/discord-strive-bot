use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<ApiMessage>,
}

#[derive(Serialize, Clone)]
pub struct ApiMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Deserialize)]
pub struct ClaudeResponse {
    pub content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
pub struct ClaudeContent {
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct ContextConfig {
    pub files: Vec<FileRef>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileRef {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
}
