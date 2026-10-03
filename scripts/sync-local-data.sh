#!/usr/bin/env bash
#
# Copy the real Warframe/AlecaFrame data into local/warframe/ inside this repo,
# and decrypt the inventory alongside it.
#
# Why in-repo rather than reading C:\ directly: the project directory is bind
# mounted into the DDEV container, so anything here is visible from both the host
# and the container and survives container rebuilds. Reading through a /mnt/c
# bind mount instead ties the setup to WSL and to one machine, and the container's
# own home directory is ephemeral.
#
# local/ is gitignored. The files it holds are account data - your inventory, and
# the names of every player you have traded or chatted with - so they must never
# be committed.
#
# Decryption needs the AlecaFrame AES key and IV, which are not stored in this
# repository (see docs/FORK.md). Supply them, or put them in the project-root
# config.json and let the app decrypt instead:
#
#   WF_DECRYPT_KEY=<32 hex> WF_DECRYPT_IV=<32 hex> scripts/sync-local-data.sh
#
# Run from WSL, on the host - not inside the container.
#
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$HERE/local/warframe"
# shellcheck disable=SC1091
[ -f "$HERE/.env.local" ] && . "$HERE/.env.local"

if [ -n "${LOCALAPPDATA_WSL:-}" ]; then
  LOCALAPPDATA="$LOCALAPPDATA_WSL"
else
  WINUSER="$(cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d '\r\n' || true)"
  [ -n "$WINUSER" ] || WINUSER="$USER"
  LOCALAPPDATA="/mnt/c/Users/$WINUSER/AppData/Local"
fi

mkdir -p "$DEST"
echo "source: $LOCALAPPDATA"
echo "target: $DEST"
echo

copy() {
  local src="$1" name="$2"
  if [ ! -f "$src" ]; then
    printf '  %-16s not found at %s\n' "$name" "$src"
    return 1
  fi
  cp "$src" "$DEST/$name"
  printf '  %-16s %s bytes, modified %s\n' "$name" \
    "$(stat -c %s "$DEST/$name")" "$(stat -c %y "$src" | cut -d. -f1)"
}

copy "$LOCALAPPDATA/AlecaFrame/lastData.dat" "lastData.dat" || true
# EE.log is live: the game appends to it continuously, so this is a snapshot.
# Re-run this script to refresh it.
copy "$LOCALAPPDATA/Warframe/EE.log" "EE.log" || true

if [ -n "${WF_DECRYPT_KEY:-}" ] && [ -n "${WF_DECRYPT_IV:-}" ] && [ -f "$DEST/lastData.dat" ]; then
  echo
  echo "decrypting inventory..."
  "$HERE/scripts/decrypt-alecaframe.sh" "$DEST/lastData.dat" "$DEST/inventory.json"
else
  echo
  echo "  inventory.json  skipped - WF_DECRYPT_KEY/WF_DECRYPT_IV not set."
  echo "                  Not required if the app decrypts via config.json."
fi

echo
echo "Paths to use, from inside the container:"
echo "  /var/www/html/local/warframe/lastData.dat    (AlecaFrame source)"
echo "  /var/www/html/local/warframe/inventory.json  (File source)"
echo "  /var/www/html/local/warframe/EE.log          (log parser)"
