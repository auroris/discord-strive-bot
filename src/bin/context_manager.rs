use anyhow::{anyhow, Result};
use discord_strive::claude::history::context_dir;
use discord_strive::claude::types::{ContextConfig, FileRef};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

const API_BASE: &str = "https://api.anthropic.com/v1/files";

// --- API response types ---

#[derive(Deserialize)]
struct FileMetadata {
    id: String,
    filename: String,
    size_bytes: u64,
}

#[derive(Deserialize)]
struct FileListResponse {
    data: Vec<FileMetadata>,
    has_more: Option<bool>,
    last_id: Option<String>,
}

#[derive(Deserialize)]
struct DeletedFile {
    id: String,
}

// --- Helpers ---

fn api_key() -> Result<String> {
    env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow!("ANTHROPIC_API_KEY not set"))
}

fn context_json_path() -> String {
    format!("{}/context.json", context_dir())
}

fn read_context() -> Result<ContextConfig> {
    let path = context_json_path();
    match fs::read_to_string(&path) {
        Ok(s) => Ok(serde_json::from_str(&s)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(ContextConfig { files: Vec::new() })
        }
        Err(e) => Err(e.into()),
    }
}

fn write_context(config: &ContextConfig) -> Result<()> {
    let path = context_json_path();
    let dir = context_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&path, format!("{}\n", json))?;
    Ok(())
}

fn make_client() -> Client {
    Client::new()
}

fn api_headers(req: reqwest::blocking::RequestBuilder, api_key: &str) -> reqwest::blocking::RequestBuilder {
    req.header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "files-api-2025-04-14")
}

// --- Subcommands ---

fn cmd_upload(file_path: &str, title: Option<&str>) -> Result<()> {
    let api_key = api_key()?;
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(anyhow!("File not found: {}", file_path));
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let file_bytes = fs::read(path)?;
    let part = reqwest::blocking::multipart::Part::bytes(file_bytes)
        .file_name(filename.clone())
        .mime_str("application/octet-stream")?;
    let form = reqwest::blocking::multipart::Form::new().part("file", part);

    let client = make_client();
    let response = api_headers(client.post(API_BASE), &api_key)
        .multipart(form)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(anyhow!("Upload failed ({}): {}", status, body));
    }

    let metadata: FileMetadata = response.json()?;
    let display_title = title.unwrap_or(&filename);

    let mut config = read_context()?;
    config.files.push(FileRef {
        id: metadata.id.clone(),
        title: Some(display_title.to_string()),
    });
    write_context(&config)?;

    println!("Uploaded: {} ({} bytes)", metadata.filename, metadata.size_bytes);
    println!("File ID: {}", metadata.id);
    println!("Added to context.json as \"{}\"", display_title);

    Ok(())
}

fn cmd_verify() -> Result<()> {
    let api_key = api_key()?;
    let config = read_context()?;

    if config.files.is_empty() {
        println!("context.json has no files.");
        return Ok(());
    }

    // Collect all remote file IDs with pagination
    let client = make_client();
    let mut remote_ids = HashSet::new();
    let mut after_id: Option<String> = None;

    loop {
        let mut url = format!("{}?limit=1000", API_BASE);
        if let Some(ref cursor) = after_id {
            url.push_str(&format!("&after_id={}", cursor));
        }

        let response = api_headers(client.get(&url), &api_key).send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("List files failed ({}): {}", status, body));
        }

        let list: FileListResponse = response.json()?;
        for file in &list.data {
            remote_ids.insert(file.id.clone());
        }

        if list.has_more == Some(true) {
            after_id = list.last_id;
        } else {
            break;
        }
    }

    // Partition into kept and removed
    let mut kept = Vec::new();
    let mut removed = Vec::new();

    for file in &config.files {
        if remote_ids.contains(&file.id) {
            kept.push(file.clone());
        } else {
            removed.push(file.clone());
        }
    }

    if removed.is_empty() {
        println!("All {} file(s) verified — all exist remotely.", kept.len());
        return Ok(());
    }

    // Write updated config
    let updated = ContextConfig { files: kept.clone() };
    write_context(&updated)?;

    println!("Kept {} file(s):", kept.len());
    for f in &kept {
        println!("  {} — {}", f.id, f.title.as_deref().unwrap_or("(untitled)"));
    }
    println!("Removed {} stale entry/entries:", removed.len());
    for f in &removed {
        println!("  {} — {}", f.id, f.title.as_deref().unwrap_or("(untitled)"));
    }

    Ok(())
}

fn cmd_delete(file_id: &str) -> Result<()> {
    let api_key = api_key()?;
    let client = make_client();

    let url = format!("{}/{}", API_BASE, file_id);
    let response = api_headers(client.delete(&url), &api_key).send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(anyhow!("Delete failed ({}): {}", status, body));
    }

    let deleted: DeletedFile = response.json()?;

    // Remove from context.json
    let mut config = read_context()?;
    let before_len = config.files.len();
    config.files.retain(|f| f.id != deleted.id);
    let removed_count = before_len - config.files.len();
    write_context(&config)?;

    println!("Deleted file: {}", deleted.id);
    if removed_count > 0 {
        println!("Removed from context.json.");
    } else {
        println!("(was not in context.json)");
    }

    Ok(())
}

// --- Usage ---

fn print_usage() {
    eprintln!("Usage: context-manager <command> [args]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  upload <file_path> [--title <title>]   Upload a file and add to context.json");
    eprintln!("  verify                                 Check remote files, remove stale entries");
    eprintln!("  delete <file_id>                       Delete a file and remove from context.json");
}

// --- Main ---

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "upload" => {
            if args.len() < 3 {
                eprintln!("Usage: context-manager upload <file_path> [--title <title>]");
                std::process::exit(1);
            }
            let file_path = &args[2];
            let title = parse_flag(&args[3..], "--title");
            cmd_upload(file_path, title.as_deref())
        }
        "verify" => cmd_verify(),
        "delete" => {
            if args.len() < 3 {
                eprintln!("Usage: context-manager delete <file_id>");
                std::process::exit(1);
            }
            cmd_delete(&args[2])
        }
        other => {
            eprintln!("Unknown command: {}", other);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn parse_flag(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        }
    }
    None
}
