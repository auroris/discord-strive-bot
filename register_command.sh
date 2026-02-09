#!/bin/bash

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

curl -X POST \
  "https://discord.com/api/v10/applications/${APP_ID}/commands" \
  -H "Authorization: Bot ${BOT_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "strive",
    "description": "Talk to Strive",
    "integration_types": [0, 1],
    "contexts": [0, 1, 2],
    "options": [
      {
        "name": "text",
        "description": "What do you want to say?",
        "type": 3,
        "required": true
      }
    ]
  }'