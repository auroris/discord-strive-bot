# Strive — A CGI-Based Discord Bot Powered by Claude

Strive is a Discord bot that responds to slash commands using Anthropic's Claude API. It runs as a **CGI script** behind nginx + fcgiwrap rather than as a persistent server process, making it ideal for resource-constrained machines where a constantly-running Node.js or Python server would be too heavy on RAM.

When a user runs `/strive`, the CGI handler verifies the request, immediately defers the Discord response, and spawns a background process that calls the Claude API and delivers the reply — all without a long-lived server process sitting in memory.

## Architecture

```
Discord User (/strive "hello")
        │
        ▼
  Discord API ──── HTTPS POST ────▶ nginx
                                      │
                                      ▼
                                  fcgiwrap
                                      │
                                      ▼
                                 cgi-handler
                                  │       │
                          Verify Ed25519  Return deferred
                          signature       response (type 5)
                                  │
                                  ▼
                          fork + exec callback-handler
                          (runs in background, CGI exits)
                                  │
                                  ▼
                           callback-handler
                            │           │
                    Load history    Load context files
                    (per user/      (from Anthropic
                     channel)        Files API refs)
                            │
                            ▼
                      Call Claude API
                            │
                            ▼
                   PATCH Discord webhook
                   (edit deferred message)
                            │
                            ▼
                   Discord User sees response
```

### Why CGI?

A typical Discord bot runs as a persistent process, maintaining a WebSocket connection to Discord's gateway. This works well on machines with RAM to spare, but on a VPS or single-board computer, a Node.js runtime idling at 50-100+ MB of RSS is wasteful when the bot only handles occasional interactions.

Discord supports an [Interactions Endpoint](https://discord.com/developers/docs/interactions/overview#setting-up-an-endpoint) mode where interactions are delivered via HTTP POST to a URL you control. By using this mode with CGI:

- **Zero idle memory** — nothing runs between requests
- **nginx + fcgiwrap** handle the HTTP plumbing
- **Rust binaries** start fast and use minimal memory
- The callback handler runs only for the few seconds it takes to call Claude and respond

### Binaries

| Binary | Purpose |
|---|---|
| `cgi-handler` | CGI entry point. Verifies Discord signatures, returns a deferred response, and spawns the callback handler. |
| `callback-handler` | Background worker. Loads conversation history and context, calls Claude, and patches the Discord message with the response. |
| `context-manager` | CLI utility for managing files uploaded to Anthropic's Files API (upload, verify, delete). |

## Prerequisites

- **Rust** (stable, edition 2024) — [Install via rustup](https://rustup.rs/)
- **Linux** — The `fork` crate is used for daemonization, so this only runs on Linux
- **nginx** with `fcgiwrap` — to serve the CGI handler
- A **Discord application** with an Interactions Endpoint URL configured
- An **Anthropic API key** for Claude

## Setup

### 1. Clone and Build

```bash
git clone https://github.com/yourusername/discord-strive-bot.git
cd discord-strive-bot
cargo build --release
```

The binaries will be at:
- `target/release/cgi-handler`
- `target/release/callback-handler`
- `target/release/context-manager`

### 2. Create Your Discord Application

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications) and create a new application.
2. Note your **Application ID** and **Public Key** from the General Information page.
3. Under **Bot**, create a bot and note the **Bot Token**.
4. Under **Installation**, configure the install link with the `applications.commands` scope.

### 3. Configure Environment Variables

Copy the example env file and fill it in:

```bash
cp .env.example .env
```

Edit `.env`:

```
BOT_TOKEN=your_discord_bot_token
APP_ID=your_discord_application_id
```

These are used by the helper scripts (`register_command.sh`). The CGI handler itself receives its configuration through nginx fastcgi_param directives (see below).

### 4. Register the Slash Command

```bash
chmod +x register_command.sh
./register_command.sh
```

This registers the `/strive` slash command with Discord. You only need to do this once (or when you change the command definition).

### 5. Install and Configure fcgiwrap

```bash
sudo apt install fcgiwrap
```

fcgiwrap should start automatically and create a socket. Verify it's running:

```bash
sudo systemctl status fcgiwrap
ls -la /var/run/fcgiwrap.socket
```

### 6. Configure nginx

Add a location block to your nginx site configuration. This block passes Discord interaction requests to the CGI handler via fcgiwrap:

```nginx
location /discord/interactions {
    include fastcgi_params;

    # Discord signature headers (required for verification)
    fastcgi_param HTTP_X_SIGNATURE_ED25519 $http_x_signature_ed25519;
    fastcgi_param HTTP_X_SIGNATURE_TIMESTAMP $http_x_signature_timestamp;

    # Bot configuration — set these to your own values
    fastcgi_param DISCORD_PUBLIC_KEY "your_discord_public_key_hex";
    fastcgi_param ANTHROPIC_API_KEY "your_anthropic_api_key";
    fastcgi_param CALLBACK_BINARY_PATH "/path/to/target/release/callback-handler";
    fastcgi_param STRIVE_CONTEXT_PATH "/path/to/discord-strive-bot/context/";

    # CGI script path
    fastcgi_param SCRIPT_FILENAME /path/to/target/release/cgi-handler;

    # Pass to fcgiwrap
    fastcgi_pass unix:/var/run/fcgiwrap.socket;
}
```

| Parameter | Description |
|---|---|
| `DISCORD_PUBLIC_KEY` | Your Discord application's public key (hex string from the Developer Portal). Used to verify request signatures. |
| `ANTHROPIC_API_KEY` | Your Anthropic API key for Claude API calls. |
| `CALLBACK_BINARY_PATH` | Absolute path to the `callback-handler` binary. |
| `STRIVE_CONTEXT_PATH` | Absolute path to the `context/` directory containing history and context files. |

Test and reload nginx:

```bash
sudo nginx -t
sudo systemctl reload nginx
```

### 7. Set the Interactions Endpoint URL

In the Discord Developer Portal, under your application's **General Information**, set the **Interactions Endpoint URL** to:

```
https://yourdomain.com/discord/interactions
```

Discord will send a PING to verify your endpoint. The CGI handler responds to PINGs automatically — if verification succeeds, you're good to go.

### 8. Context Files (Optional)

The `context/` directory supports optional configuration:

**`context/context.json`** — References to files uploaded via Anthropic's Files API. These are included as document blocks in the first message of each conversation, giving Claude persistent reference material.

```json
{
  "files": [
    {
      "id": "file_abc123",
      "title": "My Reference Document"
    }
  ]
}
```

Use the `context-manager` binary to manage these:

```bash
# Upload a file and add it to context.json
./target/release/context-manager upload ./my-document.txt --title "My Reference Document"

# Verify all referenced files still exist on the API
./target/release/context-manager verify

# Delete a file from the API and context.json
./target/release/context-manager delete file_abc123
```

The `ANTHROPIC_API_KEY` environment variable must be set for context-manager commands.

**`context/system.md`** — Optional system prompt extension. If this file exists, its contents are appended to the base system prompt. Use this to customize Strive's personality or give it additional instructions without modifying code.

## Conversation History

Strive maintains per-user, per-channel conversation history as JSONL files in the context directory:

```
context/{channel_id}_{user_id}.jsonl
```

Each line is a JSON object with `role` ("user" or "assistant") and `content` fields. History is capped at 50 messages — older messages are pruned automatically.

To reset a user's conversation, delete the corresponding `.jsonl` file.

## Debugging

Set the `STRIVE_DEBUG` environment variable to get detailed error output in Discord messages. You can add this as another `fastcgi_param` in your nginx config:

```nginx
fastcgi_param STRIVE_DEBUG "1";
```

This will include the full Claude API request JSON in error messages, which is helpful for diagnosing issues but should be disabled in production.

## Project Structure

```
discord-strive-bot/
├── src/
│   ├── lib.rs                    # Shared library exports
│   ├── bin/
│   │   ├── cgi_handler.rs        # CGI entry point
│   │   ├── callback_handler.rs   # Async Claude + Discord worker
│   │   └── context_manager.rs    # File management CLI
│   ├── discord/
│   │   ├── types.rs              # Discord API types
│   │   ├── verify.rs             # Ed25519 signature verification
│   │   └── webhook.rs            # Discord webhook responses
│   └── claude/
│       ├── api.rs                # Claude API client
│       ├── types.rs              # Claude request/response types
│       ├── history.rs            # Conversation history (JSONL)
│       └── context.rs            # System extension + file context loading
├── context/
│   ├── context.json              # File references for Claude API
│   └── system.md                 # Optional system prompt extension
├── register_command.sh           # Discord slash command registration
├── Cargo.toml
└── .env.example
```

## License

TBD
