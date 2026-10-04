"""Write the fork's settings into the project-root config.json.

Takes the values from the environment so they are never written into a shell
history or into the repository. config.json is gitignored.

    WF_DECRYPT_KEY=... WF_DECRYPT_IV=... python3 scripts/set-config.py
    QF_API_URL=http://localhost:6969 python3 scripts/set-config.py

An installed Windows build has no project root to find config.json in, so it
reads one from its app-data directory instead. Copy the file there with:

    python3 scripts/set-config.py --install

Override the detected Windows profile with LOCALAPPDATA_WSL if it picks the
wrong one.

The keys are given as 32 hex characters, the way every reference writes them,
and stored as the sixteen byte values they stand for - so what is in the file
is what the cipher uses rather than an encoding of it.

Only the variables you set are written; the rest of config.json is preserved.
"""

import json
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PATH = os.path.join(HERE, "config.json")

APP_ID = "dev.thesneakattack.quantframe"


def local_appdata():
    """%LOCALAPPDATA% as seen from WSL, the way seed-from-upstream.sh finds it."""
    override = os.environ.get("LOCALAPPDATA_WSL")
    if override:
        return override
    try:
        user = subprocess.run(
            ["cmd.exe", "/c", "echo %USERNAME%"],
            capture_output=True, text=True, timeout=10,
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        user = ""
    return os.path.join("/mnt/c/Users", user or os.environ.get("USER", ""), "AppData", "Local")


def install():
    """Put config.json where an installed build will look for it."""
    if not os.path.exists(PATH):
        sys.exit(f"No {PATH} to install. Write one first (see --help).")
    target_dir = os.path.join(local_appdata(), APP_ID)
    if not os.path.isdir(target_dir):
        sys.exit(
            f"{target_dir} does not exist.\n"
            "Install and run the app once so it creates its data directory, "
            "or set LOCALAPPDATA_WSL if the Windows profile was detected wrongly."
        )
    target = os.path.join(target_dir, "config.json")
    shutil.copyfile(PATH, target)
    print(f"  ok    {target}")


if "--install" in sys.argv:
    install()
    raise SystemExit(0)

KEY_FIELDS = ("wf_decrypt_key", "wf_decrypt_iv")
ENV = {
    "wf_decrypt_key": "WF_DECRYPT_KEY",
    "wf_decrypt_iv": "WF_DECRYPT_IV",
    "qf_api_url": "QF_API_URL",
}


def to_bytes(var, value):
    """The sixteen bytes a 32-character hex key stands for."""
    if len(value) != 32 or any(c not in "0123456789abcdefABCDEF" for c in value):
        sys.exit(f"{var} must be exactly 32 hex characters, got {len(value)}")
    return list(bytes.fromhex(value))


updates = {}
for field, var in ENV.items():
    value = os.environ.get(var)
    if value is None:
        continue
    value = value.strip()
    updates[field] = to_bytes(var, value) if field in KEY_FIELDS else value

if not updates:
    sys.exit("Nothing to do: set at least one of " + ", ".join(ENV.values()))

config = {}
if os.path.exists(PATH):
    with open(PATH) as fh:
        config = json.load(fh)
config.update(updates)

with open(PATH, "w") as fh:
    json.dump(config, fh, indent=2)
    fh.write("\n")

print(f"  ok    {PATH}")
for field, value in updates.items():
    shown = f"{len(value)} bytes" if field in KEY_FIELDS else value
    print(f"        {field}: {shown}")
