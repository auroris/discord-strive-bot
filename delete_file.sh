#!/bin/bash

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Verify API key is set
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "Error: ANTHROPIC_API_KEY not found in environment"
    exit 1
fi

# Verify file argument provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <file_id>"
    exit 1
fi

FILE_ID="$1"

curl -X DELETE https://api.anthropic.com/v1/files/$FILE_ID \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "anthropic-beta: files-api-2025-04-14"