#!/bin/sh
set -e


if [ -x "$YTDLP_PATH" ]; then
  echo "Generating supported sites list..."
  
  # Centralized regex configuration for unsupported extractors
  # - CURRENTLY BROKEN : Extractors officially marked as broken in yt-dlp.
  # - youtube          : disabled in this deployment due to local legal/compliance policy (remove from UNSUPPORTED_REGEX to enable).
  UNSUPPORTED_REGEX="CURRENTLY BROKEN|youtube"
  
  "$YTDLP_PATH" --list-extractors > /tmp/all_extractors.txt || true
  
  grep -E -i -v "$UNSUPPORTED_REGEX" /tmp/all_extractors.txt > /home/app/sites.txt || true
  grep -E -i "$UNSUPPORTED_REGEX" /tmp/all_extractors.txt > /home/app/unsupported.txt || true
  
  rm -f /tmp/all_extractors.txt
fi

echo "Starting Rust API"
exec /usr/local/bin/app
