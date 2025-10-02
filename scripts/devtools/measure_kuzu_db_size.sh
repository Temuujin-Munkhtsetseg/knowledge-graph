#!/bin/bash

# Run gkg index, extract database.kz path, and show file size as JSON
DB_PATH=$(cargo run --release --bin gkg index $1 2>/dev/null | \
  grep -o '/[^[:space:]]*database\.kz' | \
  head -1)

if [ -z "$DB_PATH" ]; then
  echo '{"error": "database.kz path not found"}'
  exit 1
fi

SIZE_BYTES=$(stat -f%z "$DB_PATH" 2>/dev/null || stat -c%s "$DB_PATH" 2>/dev/null)
SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1024 / 1024" | bc)
SIZE_GB=$(echo "scale=2; $SIZE_BYTES / 1024 / 1024 / 1024" | bc)

echo "{
  \"path\": \"$1\",
  \"db_path\": \"$DB_PATH\",
  \"size_bytes\": $SIZE_BYTES,
  \"size_mb\": $SIZE_MB,
  \"size_gb\": $SIZE_GB
}"
