#!/usr/bin/env bash
#
# Seed this fork's Windows app-data directory from an existing official Quantframe
# install, so the first run starts with your real login, settings and transaction
# history instead of an empty database.
#
# The rebrand gave this build its own bundle identifier
# (dev.thesneakattack.quantframe), which is what lets it coexist with official
# Quantframe. The cost is that it starts from nothing. This copies the state over.
#
# Run from WSL. Refuses to overwrite an existing non-empty target unless --force.
#
#   scripts/seed-from-upstream.sh --dry-run
#   scripts/seed-from-upstream.sh
#
set -euo pipefail

SRC_ID="dev.kenya.quantframe"
DST_ID="dev.thesneakattack.quantframe"
DRY_RUN=0
FORCE=0

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --force)   FORCE=1 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

# Locate %LOCALAPPDATA% from WSL. Override with LOCALAPPDATA_WSL if autodetection
# picks the wrong profile.
if [ -n "${LOCALAPPDATA_WSL:-}" ]; then
  LOCALAPPDATA="$LOCALAPPDATA_WSL"
else
  WINUSER="$(cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d '\r\n' || true)"
  [ -n "$WINUSER" ] || WINUSER="$USER"
  LOCALAPPDATA="/mnt/c/Users/$WINUSER/AppData/Local"
fi

SRC="$LOCALAPPDATA/$SRC_ID"
DST="$LOCALAPPDATA/$DST_ID"

echo "source: $SRC"
echo "target: $DST"
echo

[ -d "$SRC" ] || { echo "ERROR: no official Quantframe install found at $SRC" >&2; exit 1; }

if [ -d "$DST" ] && [ -n "$(ls -A "$DST" 2>/dev/null)" ] && [ "$FORCE" = 0 ]; then
  echo "ERROR: $DST already exists and is not empty." >&2
  echo "       Refusing to overwrite - this build may already hold state you want." >&2
  echo "       Re-run with --force if you really mean to replace it." >&2
  exit 1
fi

# Copied: everything that carries real state forward.
#   quantframeV2.sqlite  stock, transactions, wishlist
#   settings.json        live scraper config, log path, notifications
#   auth.json            QF + WFM session, so you are not asked to log in again
#   cache/, cache_version.json   the item cache, so first start is not a full download
#
# Not copied, deliberately:
#   EBWebView/           WebView2 profile, regenerates and is tied to the old app id
#   logs/, cached_logs/  the other install's logs
#   *_backup             this build writes its own backup on every start
ITEMS=(quantframeV2.sqlite settings.json auth.json cache_version.json .cookies cache)

if [ "$DRY_RUN" = 1 ]; then
  echo "DRY RUN - nothing will be written"
  echo
fi

[ "$DRY_RUN" = 1 ] || mkdir -p "$DST"

for item in "${ITEMS[@]}"; do
  if [ ! -e "$SRC/$item" ]; then
    printf '  %-22s %s\n' "$item" "(absent in source, skipped)"
    continue
  fi
  size="$(du -sh "$SRC/$item" 2>/dev/null | cut -f1)"
  if [ "$DRY_RUN" = 1 ]; then
    printf '  %-22s would copy (%s)\n' "$item" "$size"
  else
    cp -r "$SRC/$item" "$DST/"
    printf '  %-22s copied (%s)\n' "$item" "$size"
  fi
done

echo
if [ "$DRY_RUN" = 1 ]; then
  echo "Dry run complete. Re-run without --dry-run to apply."
else
  echo "Done. Quantframe Sneak will start with your existing login, settings and history."
  echo
  echo "Reminder: do not run both builds with the live scraper enabled at the same"
  echo "time - they manage the same warframe.market orders and will fight."
fi
