#!/usr/bin/env bash
# Galleon Fleet - First Boot & User Data Migration Tool
# Migrates ~/.clawcrew/* to ~/.config/galleon/* atomically with .migrated marker.

set -euo pipefail

LEGACY_DIR="${1:-$HOME/.clawcrew}"
TARGET_DIR="${2:-${XDG_CONFIG_HOME:-$HOME/.config}/galleon}"

echo "=== Galleon Data Migration ==="
echo "Legacy dir: $LEGACY_DIR"
echo "Target dir: $TARGET_DIR"

if [[ ! -d "$LEGACY_DIR" ]]; then
    echo "No legacy directory found at $LEGACY_DIR. Nothing to migrate."
    exit 0
fi

if [[ -f "$LEGACY_DIR/.migrated" ]]; then
    echo "Legacy directory is already marked as migrated (.migrated exists). Skipping."
    exit 0
fi

if [[ -d "$TARGET_DIR" ]] && [[ "$(ls -A "$TARGET_DIR" 2>/dev/null | grep -v '^\.migrated$' || true)" != "" ]]; then
    echo "Target directory $TARGET_DIR already exists and contains files. Skipping migration to prevent overwrite."
    exit 0
fi

TARGET_PARENT="$(dirname "$TARGET_DIR")"
mkdir -p "$TARGET_PARENT"

TMP_DIR="$(mktemp -d "$TARGET_PARENT/.galleon_migration_tmp_XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Copying files from $LEGACY_DIR to staging $TMP_DIR..."
cp -a "$LEGACY_DIR"/. "$TMP_DIR"/

MARKER_CONTENT="migrated = true\nsource = \"$LEGACY_DIR\"\ntarget = \"$TARGET_DIR\"\ntimestamp = \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"\n"
printf "$MARKER_CONTENT" > "$TMP_DIR/.migrated"

if [[ -d "$TARGET_DIR" ]]; then
    rm -rf "$TARGET_DIR"
fi

mv "$TMP_DIR" "$TARGET_DIR"
trap - EXIT

printf "$MARKER_CONTENT" > "$LEGACY_DIR/.migrated"
echo "Migration completed successfully. Target: $TARGET_DIR"
