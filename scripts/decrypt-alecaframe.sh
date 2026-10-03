#!/usr/bin/env bash
#
# Decrypt AlecaFrame's lastData.dat into plain JSON, for the WF Inventory "File"
# inventory source.
#
# Why this exists: the AlecaFrame source asks api.quantframe.app for the AES key
# and IV, and that endpoint returns 403 on accounts without the entitlement (see
# docs/superpowers/research/2026-10-03-self-hosting-the-quantframe-api.md). The
# keys are static, so decryption can be done locally instead.
#
# The key and IV are deliberately NOT stored in this repository. Supply them:
#
#   WF_DECRYPT_KEY=<32 hex chars> WF_DECRYPT_IV=<32 hex chars> \
#     scripts/decrypt-alecaframe.sh
#
# or put them in .env.local beside this repo, which is gitignored.
#
# Usage:
#   scripts/decrypt-alecaframe.sh [source lastData.dat] [destination .json]
#
# Defaults read and write inside %LOCALAPPDATA%\AlecaFrame on the Windows side.
#
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
[ -f "$HERE/.env.local" ] && . "$HERE/.env.local"

: "${WF_DECRYPT_KEY:?set WF_DECRYPT_KEY to the 32-hex-character AES key}"
: "${WF_DECRYPT_IV:?set WF_DECRYPT_IV to the 32-hex-character AES IV}"

if [ -n "${LOCALAPPDATA_WSL:-}" ]; then
  LOCALAPPDATA="$LOCALAPPDATA_WSL"
else
  WINUSER="$(cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d '\r\n' || true)"
  [ -n "$WINUSER" ] || WINUSER="$USER"
  LOCALAPPDATA="/mnt/c/Users/$WINUSER/AppData/Local"
fi

SRC="${1:-$LOCALAPPDATA/AlecaFrame/lastData.dat}"
DST="${2:-$LOCALAPPDATA/AlecaFrame/inventory.json}"

[ -f "$SRC" ] || { echo "ERROR: no lastData.dat at $SRC" >&2; exit 1; }

echo "source: $SRC"
echo "        $(stat -c %s "$SRC") bytes, modified $(stat -c %y "$SRC" | cut -d. -f1)"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

# AES-128-CBC. AlecaFrame pads with PKCS7, which openssl strips by default.
openssl enc -d -aes-128-cbc -K "$WF_DECRYPT_KEY" -iv "$WF_DECRYPT_IV" -in "$SRC" -out "$tmp"

# Validate before writing, so a wrong key fails loudly instead of leaving the app
# to silently ignore a garbage file.
python3 - "$tmp" <<'PY'
import json, sys

with open(sys.argv[1], "rb") as fh:
    raw = fh.read().decode("utf-8", "ignore")

# Any padding leaves bytes after the final brace; the app's parser trims the same way.
end = raw.rfind("}")
if end == -1:
    sys.exit("decryption produced no JSON object - is the key correct?")
raw = raw[: end + 1]

try:
    data = json.loads(raw)
except json.JSONDecodeError as exc:
    sys.exit(f"decryption did not produce valid JSON ({exc}) - is the key correct?")

expected = ("Upgrades", "RawUpgrades", "PremiumCredits", "TradesRemaining")
missing = [field for field in expected if field not in data]
if missing:
    sys.exit(f"decrypted, but expected fields are absent: {missing}")

print(f"        {len(data)} top-level fields, {len(data['Upgrades'])} Upgrades (rivens/mods)")

with open(sys.argv[1], "w") as fh:
    json.dump(data, fh)
PY

cp "$tmp" "$DST"
echo "wrote:  $DST"
echo "        $(stat -c %s "$DST") bytes"
echo
echo "Set WF Inventory to the File source and point it at that path."
echo "Re-run this whenever AlecaFrame refreshes lastData.dat, or prefer the"
echo "built-in local-key support so the app decrypts it itself (docs/FORK.md)."
