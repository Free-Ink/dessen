#!/usr/bin/env python3
"""Report drift between the FreeInk SDK and what dessen exposes.

This does NOT auto-generate calls. Exposing an SDK method means choosing an
app-facing shape and writing the clip/pin/range-check that makes app input
safe (see AGENTS.md rule 2) — a judgment step, not a mechanical one. What this
tool does is tell you what is new and un-exposed, and whether the binding still
builds against the SDK you have, so nothing drifts silently.

Usage: python3 tools/sync_check.py [--freeink-sdk DIR]
Exit status: 0 if in sync, 1 if there is new un-exposed surface or a pin drift.
"""
import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PIN = ROOT / "abi" / ".freeink-sync"  # the SDK commit dessen was last synced to
IGNORE = ROOT / "abi" / ".sync-ignore"  # methods deliberately not exposed

# SDK surfaces worth tracking: (header, class, what exposing one means).
SURFACES = [
    ("libs/ui/FreeInkUI/include/FreeInkUICore.h", "DrawTarget", "drawing call"),
]


def virtual_methods(header: Path, cls: str):
    """Names of the virtual methods declared in `class cls` in `header`."""
    text = header.read_text()
    m = re.search(rf"class {cls}\b.*?\{{(.*?)\n\}};", text, re.S)
    if not m:
        return set()
    body = m.group(1)
    return set(re.findall(r"virtual\s+[\w:<>,&*\s]+?\b(\w+)\s*\(", body)) - {cls, "~" + cls}


def exposed_methods():
    """Method names called on `target` anywhere in the binding or .api — a call
    reaches the SDK either straight from a .api host statement or through a
    helper in DessenFreeInk.cpp (drawText -> target.text, etc.)."""
    text = (ROOT / "abi" / "freeink.api").read_text()
    text += (ROOT / "freeink" / "DessenFreeInk.cpp").read_text()
    return set(re.findall(r"\btarget\.(\w+)\s*\(", text))


def git_sha(repo: Path):
    try:
        return subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"],
                              capture_output=True, text=True, check=True).stdout.strip()
    except Exception:
        return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--freeink-sdk", default=str(ROOT.parent / "freeink-sdk"))
    args = ap.parse_args()
    sdk = Path(args.freeink_sdk)
    if not sdk.is_dir():
        sys.exit(f"freeink-sdk not found at {sdk} (pass --freeink-sdk DIR)")

    drift = False
    exposed = exposed_methods()
    if IGNORE.exists():
        for line in IGNORE.read_text().splitlines():
            name = line.split("#")[0].strip()
            if name:
                exposed.add(name)
    for rel, cls, kind in SURFACES:
        header = sdk / rel
        if not header.exists():
            print(f"? {rel} missing — SDK moved it; update SURFACES in this script")
            drift = True
            continue
        methods = virtual_methods(header, cls)
        unexposed = sorted(methods - exposed)
        if unexposed:
            drift = True
            print(f"NEW in {cls}: {', '.join(unexposed)}")
            print(f"  -> consider a {kind} in abi/freeink.api for each (AGENTS.md recipe).")
    # Methods the .api references that no longer exist = a rename/removal to fix.
    all_methods = set()
    for rel, cls, _ in SURFACES:
        if (sdk / rel).exists():
            all_methods |= virtual_methods(sdk / rel, cls)
    stale = sorted(exposed - all_methods)
    if stale:
        drift = True
        print(f"STALE: abi/freeink.api calls target.{{{', '.join(stale)}}} which the SDK "
              f"no longer declares — the binding may not build. Fix the call(s).")

    # Pin drift: did the SDK commit move since the last recorded sync?
    sha = git_sha(sdk)
    last = PIN.read_text().strip() if PIN.exists() else None
    if sha and last and sha != last:
        print(f"SDK moved since last sync: {last[:10]} -> {sha[:10]}. "
              f"Review new surface above, then `make sync-accept` to record it.")
    elif sha and not last:
        print(f"No sync recorded yet. After reviewing, `make sync-accept` to pin {sha[:10]}.")

    if not drift:
        print("In sync: every tracked SDK surface is accounted for in abi/freeink.api.")
    return 1 if drift else 0


if __name__ == "__main__":
    sys.exit(main())
