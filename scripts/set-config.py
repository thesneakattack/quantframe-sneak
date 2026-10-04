"""Write the fork's settings into the project-root config.json.

Takes the values from the environment so they are never written into a shell
history or into the repository. config.json is gitignored.

    WF_DECRYPT_KEY=... WF_DECRYPT_IV=... python3 scripts/set-config.py
    QF_API_URL=http://localhost:6969 python3 scripts/set-config.py

The keys are given as 32 hex characters, the way every reference writes them,
and stored as the sixteen byte values they stand for - so what is in the file
is what the cipher uses rather than an encoding of it.

Only the variables you set are written; the rest of config.json is preserved.
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PATH = os.path.join(HERE, "config.json")

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
