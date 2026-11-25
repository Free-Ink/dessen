#!/usr/bin/env python3
"""Write a reading session log for `make rust` to check apps/readstats.rs against.

Usage: readstats_fixture.py <fs-root>
Dated relative to now, in the local zone, because the app asks the host clock
what "today" is. Expected: Today 30m, Last 7 days 45m, Streak 2 days,
Total 5h 45m over 32 sessions.
"""
import json, os, sys, time

now = int(os.environ.get("DESSEN_NOW") or time.time())  # the screen tests fix the clock
tz = time.localtime().tm_gmtoff // 60
DUNE, HAND = "d" * 32, "a" * 32


def session(doc, title, days_ago, secs):
    return json.dumps({"doc": doc, "start": now - days_ago * 86400, "secs": secs, "tz": tz, "from": 1000,
                       "to": 1500, "title": title, "path": "/books/x.epub"}, separators=(",", ":"))


lines = [session(DUNE, "Dune", d, 600) for d in range(39, 9, -1)]  # 30 older sessions: spans several read chunks
lines += [
    "not json",
    "x" * 3000,                                    # longer than the app's buffer
    session(HAND, 'The "Left" Hand', 1, 900),      # escaped quotes in the title
    session(DUNE, "Dune", 0, 1800),
    session(DUNE, "Dune", -3, 5000),               # dated in the future: ignored
]
os.makedirs(os.path.join(sys.argv[1], ".crosspoint"), exist_ok=True)
with open(os.path.join(sys.argv[1], ".crosspoint", "reading_sessions.jsonl"), "w") as f:
    f.write("\n".join(lines) + "\n" + session(DUNE, "Dune", 0, 9999))  # unfinished last line: ignored
