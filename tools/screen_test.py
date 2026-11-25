#!/usr/bin/env python3
"""Screen tests for the xpui apps: pictures, element positions and the screen
standards, from walks through every screen.

Usage: screen_test.py [--update] [--only APP[/WALK]] [--show] SCENARIO_FILE...
       screen_test.py --standards SCENARIO_FILE...

Each scenario file (tests/xpui/<app>.scenarios) lists walks through one app.
Every walk runs on the desktop runner (build/dessen) at each screen size, in a
fixed world (the clock, the time zone and the random numbers), drawn through
FreeInk's own framebuffer target with real fonts (DESSEN_SNAPSHOT). The screen
it ends on is compared with its goldens in tests/xpui/golden/<app>/:

  <walk>@<w>x<h>.png    the picture. Fails when a pixel of ink has no ink
                        within TOLERANCE px of it in the other picture; a diff
                        image is written to build/screen-diffs/.
  <walk>@<w>x<h>.trace  the draw calls, with where each text's words land.
                        Every text and button is found again by its label and
                        must be within TOLERANCE px of where it was, the same
                        size, and neither missing nor new. The calls themselves
                        may change (a new framework draws the same picture
                        differently): that is only a note.

  - A missing golden is written and FAILS, so nobody commits a picture they
    never looked at. Look at it, then run again.
  - --update (or UPDATE_GOLDENS=1) rewrites the goldens that differ.
  - --standards checks every walk against every standard, declared or not,
    and lists what each app still breaks. It compares no goldens and always
    succeeds.

Every frame of every walk must also stay on the screen and give each text a
box at least its font's line tall (shorter draws nothing on a device). The
screen standards an app declares (`standards = ...`) are checked on the
screen each walk ends on.

Scenario file format, one walk per line:

    # a comment
    app = nook_xpui                     the image: build/<app>.bin
    manifest = apps/nook_xpui.json      optional, passed as DESSEN_MANIFEST
    board = touch                       default board for every walk (both)
    sizes = 480x800 800x480 1448x1072   default sizes
    standards = bar taps actions        the standards checked (see STANDARDS)
    walk help [board=touch] [fs=readstats] [pushed] [skip=fit] [expect="How to"] : @? @back

Events: c b p n (Confirm, Back, Prev, Next), ^ v < > (swipes),
t<x>,<y> (a tap; x and y may be a percentage of the screen, 50%),
T<x>,<y> (a long press), @<label> or @"<two words>" (a tap on the words of
the text that reads <label> on the screen as it is then; @@ for a long press),
@back (a tap on the back arrow), type:<text> (an answer to a text prompt).

expect= is text the screen a walk ends on must show: a walk that missed its
target fails saying so, rather than as a picture that changed.

fs= gives the app a storage root: empty, readstats (tools/readstats_fixture.py)
or sync (an empty log, a CrossPoint Sync account and its server's fixtures).
"""
import difflib
import os
import re
import shlex
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RUNNER = os.path.join(ROOT, "build", "dessen")
GOLDEN = os.path.join(ROOT, "tests", "xpui", "golden")
DIFFS = os.path.join(ROOT, "build", "screen-diffs")
FIXTURE = os.path.join(ROOT, "tools", "readstats_fixture.py")

# The fixed world every walk runs in.
NOW = "1764000000"  # 2025-11-24 16:00 UTC, a Monday
SEED = "7"
DEFAULT_SIZES = ["480x800", "800x480", "1448x1072"]
# How far ink and elements may move before a screen fails: a framework that
# rounds differently moves things a pixel; anything more is a change to look at.
TOLERANCE = int(os.environ.get("SCREEN_TOLERANCE", "1"))
DIFF_LINES = 30

# ---- the trace ---------------------------------------------------------

NUM = r"(-?\d+)"
RECT = NUM + "," + NUM + " " + NUM + "x" + NUM
PATTERNS = {
    "fill": re.compile(r"fill " + RECT + r" c(\d)( d)?(?: r(\d+)/(\d+))?$"),
    "stroke": re.compile(r"stroke " + RECT + r" w(\d+) c(\d)(?: r(\d+)/(\d+))?$"),
    "line": re.compile(r"line " + NUM + "," + NUM + " - " + NUM + "," + NUM + r" w(\d+) c(\d)( d)?$"),
    "triangle": re.compile(r"triangle " + NUM + "," + NUM + " " + NUM + "," + NUM + " " + NUM + "," + NUM + r" c(\d)( d)?$"),
    "text": re.compile(r"text " + RECT + r" f(\d+) (\w+) c(\d) m(\d+)((?: (?!ink )\w+)*)(?: ink " + RECT + r")? \| (.*)$"),
    "bitmap": re.compile(r"bitmap " + RECT + r" src (\d+)x(\d+) m(\d+) c(\d)( d)?( rot\d)?$"),
    "clip": re.compile(r"clip " + RECT + "$"),
}


class Op:
    """One draw call: its kind, its bounding box, and what else it said."""

    def __init__(self, line):
        self.line = line
        self.kind = line.split(" ", 1)[0]
        m = PATTERNS.get(self.kind, re.compile("$^")).match(line)
        if not m:
            raise ValueError("unreadable draw call: " + line)
        g = m.groups()
        self.text = self.ink = None
        self.width = self.radius = self.color = 0
        self.rotated = False
        if self.kind == "line":
            x0, y0, x1, y1 = (int(v) for v in g[:4])
            self.width, self.color = int(g[4]), int(g[5])
            pad = self.width // 2
            self.rect = (min(x0, x1) - pad, min(y0, y1) - pad, abs(x1 - x0) + 1 + 2 * pad, abs(y1 - y0) + 1 + 2 * pad)
        elif self.kind == "triangle":
            xs, ys = [int(v) for v in g[0:6:2]], [int(v) for v in g[1:6:2]]
            self.color = int(g[6])
            self.rect = (min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1)
        else:
            self.rect = tuple(int(v) for v in g[:4])
            if self.kind == "fill":
                self.color, self.radius = int(g[4]), int(g[6] or 0)
            elif self.kind == "stroke":
                self.width, self.color, self.radius = int(g[4]), int(g[5]), int(g[6] or 0)
            elif self.kind == "text":
                self.font, self.align, self.color = int(g[4]), g[5], int(g[6])
                self.rotated = "rot" in g[8]
                if g[9] is not None:
                    self.ink = tuple(int(v) for v in g[9:13])
                self.text = g[13]
            elif self.kind == "bitmap":
                self.color = int(g[7])

    def words(self):
        """Where the words land: the runner's measure, else 8 px a character."""
        if self.ink is not None:
            return self.ink
        x, y, w, h = self.rect
        width = min(8 * len(self.text), w)
        left = x + (w - width if self.align == "right" else (w - width) // 2 if self.align == "center" else 0)
        return left, y, width, h


class Frame:
    def __init__(self, ops):
        self.ops = ops

    def texts(self):
        return [o for o in self.ops if o.kind == "text"]


class Run:
    """What one run of the runner printed, read into frames, and its picture."""

    def __init__(self, out, err, code, png=None):
        self.frames, self.notes, ops = [], [], []
        self.line_heights = [16, 16, 16]
        for line in out.splitlines():
            if line == "refresh":
                self.frames.append(Frame(ops))
                ops = []
            elif line.startswith("# line heights "):
                self.line_heights = [int(v) for v in line.split()[3:]]
            elif line.startswith("["):
                self.notes.append(line)
            elif line and not line.startswith("#"):
                ops.append(Op(line))
        self.pending = ops  # drawn and never refreshed
        self.code, self.err, self.png = code, err, png

    def ending(self):
        if self.notes:
            return " ".join(self.notes)
        if "FAULT" in self.err:
            return "fault: " + self.err.strip().splitlines()[-1]
        return "exit %d" % self.code if self.code else "waiting"

    def line_of(self, font):
        return self.line_heights[font] if font < len(self.line_heights) else self.line_heights[0]


# ---- running a walk ----------------------------------------------------


def run(app, env, stdin):
    full = dict(os.environ, DESSEN_TRACE="detail", DESSEN_NOW=NOW, DESSEN_SEED=SEED, TZ="UTC", **env)
    png = env.get("DESSEN_SNAPSHOT")
    if png and os.path.exists(png):
        os.remove(png)
    p = subprocess.run([RUNNER, os.path.join(ROOT, "build", app + ".bin")], input=stdin.encode(),
                       capture_output=True, env=full, timeout=120)
    picture = open(png, "rb").read() if png and os.path.exists(png) else None
    return Run(p.stdout.decode(errors="replace"), p.stderr.decode(errors="replace"), p.returncode, picture)


def middle(rect):
    x, y, w, h = rect
    return x + w // 2, y + h // 2


def point(spec, size):
    return [int(extent * float(v[:-1]) / 100) if v.endswith("%") else int(v) for v, extent in zip(spec.split(","), size)]


def find_label(frame, label):
    hits = [o for o in frame.texts() if o.text == label]
    if not hits:
        raise LookupError("no text %r on screen; it shows %s" % (label, sorted({o.text for o in frame.texts()})))
    return middle(hits[-1].words())  # the last drawn is on top


def find_back(frame, bar):
    strokes = [o for o in frame.ops if o.kind == "line" and o.width == 3 and o.rect[1] < bar][:3]
    if len(strokes) < 3:
        raise LookupError("no back arrow (three 3 px strokes) in the bar")
    x0, y0 = min(o.rect[0] for o in strokes), min(o.rect[1] for o in strokes)
    x1, y1 = max(o.rect[0] + o.rect[2] for o in strokes), max(o.rect[1] + o.rect[3] for o in strokes)
    return (x0 + x1) // 2, (y0 + y1) // 2


def drive(app, env, events, size):
    """Runs the events, resolving each tap on a label against the screen it
    lands on (by replaying the events before it), and returns the last run."""
    stdin = ""
    for event in events:
        if event.startswith("@"):
            long = event.startswith("@@")
            label = event[2:] if long else event[1:]
            screen = run(app, env, stdin)
            if not screen.frames:
                raise LookupError("%s: nothing on screen yet" % event)
            frame = screen.frames[-1]
            x, y = find_back(frame, size[1] // 4) if label == "back" else find_label(frame, label)
            stdin += "%s%d,%d;" % ("T" if long else "t", x, y)
        elif event[0] in "tT" and "," in event:
            x, y = point(event[1:], size)
            stdin += "%s%d,%d;" % (event[0], x, y)
        elif event.startswith("type:"):
            stdin += event[5:] + "\n"
        elif all(c in "cbpn^v<>" for c in event):
            stdin += event
        else:
            raise ValueError("unknown event " + event)
    return run(app, env, stdin)


def storage(kind, tmp):
    """The storage (and network) roots of fs=<kind>, built under tmp."""
    if not kind:
        return {}
    fs = os.path.join(tmp, "fs")
    os.makedirs(fs)
    fixture_env = dict(os.environ, DESSEN_NOW=NOW, TZ="UTC")
    if kind == "empty":
        return {"DESSEN_FS_ROOT": fs}
    if kind == "readstats":
        subprocess.run([sys.executable, FIXTURE, fs], check=True, env=fixture_env)
        return {"DESSEN_FS_ROOT": fs}
    if kind == "sync":  # as the Makefile's readstats_check builds it
        net = os.path.join(tmp, "net")
        api = os.path.join(net, "sync.test", "api", "v1", "stats", "sessions")
        os.makedirs(os.path.join(fs, ".crosspoint"))
        os.makedirs(api)
        with open(os.path.join(fs, ".crosspoint", "koreader.json"), "w") as f:
            f.write('{"username":"reader","password_obf":"","serverUrl":"https://sync.test/"}')
        with open(os.path.join(api, "export-link?tz=0"), "w") as f:
            f.write('{"path":"/api/v1/stats/sessions/export?export_token=t","sessions":32}')
        subprocess.run([sys.executable, FIXTURE, net], check=True, env=fixture_env)
        os.rename(os.path.join(net, ".crosspoint", "reading_sessions.jsonl"), os.path.join(api, "export?export_token=t"))
        return {"DESSEN_FS_ROOT": fs, "DESSEN_NET_ROOT": net}
    raise ValueError("unknown fs=" + kind)


# ---- pictures ----------------------------------------------------------


def read_png(data):
    """The runner's 1-bit PNG as (width, height, rows of ink): each row an int
    whose set bits are ink, the leftmost pixel the highest bit."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, idat, width = 8, b"", 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        kind, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, color = struct.unpack(">IIBB", body[:10])
            if (depth, color) != (1, 0):
                raise ValueError("not a 1-bit gray PNG")
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw, stride = zlib.decompress(idat), (width + 7) // 8
    mask = (1 << width) - 1
    rows = []
    for y in range(height):
        start = y * (stride + 1)
        if raw[start] != 0:
            raise ValueError("unexpected PNG filter")
        bits = int.from_bytes(raw[start + 1:start + 1 + stride], "big") >> (stride * 8 - width)
        rows.append(~bits & mask)  # a set bit is white in the file
    return width, height, rows


def spread(rows, width, by):
    """Every ink pixel grown into a (2 by + 1) square: where a moved pixel may land."""
    mask = (1 << width) - 1
    wide = []
    for r in rows:
        grown = r
        for k in range(1, by + 1):
            grown |= (r << k) | (r >> k)
        wide.append(grown & mask)
    out = []
    for y in range(len(rows)):
        acc = 0
        for yy in range(max(0, y - by), min(len(rows), y + by + 1)):
            acc |= wide[yy]
        out.append(acc)
    return out


def compare_pictures(old, new, by):
    """Ink in either picture with no ink within `by` px in the other: (count,
    bounding box, per-row mismatch rows) — count 0 is a match."""
    w, h, a = old
    w2, h2, b = new
    if (w, h) != (w2, h2):
        return -1, None, None
    if a == b:
        return 0, None, None
    sa, sb = spread(a, w, by), spread(b, w, by)
    bad = [(b[y] & ~sa[y]) | (a[y] & ~sb[y]) for y in range(h)]
    count = sum(r.bit_count() for r in bad)
    if not count:
        return 0, None, bad
    ys = [y for y in range(h) if bad[y]]
    lowest = min((r & -r).bit_length() - 1 for r in bad if r)
    highest = max(r.bit_length() - 1 for r in bad if r)
    box = (w - 1 - highest, ys[0], highest - lowest + 1, ys[-1] - ys[0] + 1)
    return count, box, bad


def write_diff(path, old, new, bad):
    """An RGB picture: ink in both black, only in the golden light blue, only
    now grey, and what broke the tolerance red."""
    w, h, a = old
    _, _, b = new
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        ra, rb, rbad = a[y], b[y], bad[y]
        for x in range(w):
            bit = 1 << (w - 1 - x)
            if rbad & bit:
                raw += b"\xe0\x20\x20"
            elif ra & bit and rb & bit:
                raw += b"\x00\x00\x00"
            elif ra & bit:
                raw += b"\x90\xb8\xf0"
            elif rb & bit:
                raw += b"\x90\x90\x90"
            else:
                raw += b"\xff\xff\xff"

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b""))


# ---- elements ----------------------------------------------------------


def capsules(frame):
    """Buttons: capsules (a fill or stroke rounded by half its height, the
    standards' button) with a label inside, as {rect: (label, solid)}. The
    trace cannot say what is tappable, so a badge drawn as a capsule counts."""
    found = {}
    for o in frame.ops:
        if o.kind in ("fill", "stroke") and o.radius > 0 and 2 * o.radius >= o.rect[3] - 2:
            solid = found.get(o.rect, (None, False))[1] or (o.kind == "fill" and o.color == 4)
            found[o.rect] = (None, solid)
    for t in frame.texts():
        for rect, (label, solid) in found.items():
            if label is None and inside(t.words(), rect):
                found[rect] = (t.text, solid)
    return {r: v for r, v in found.items() if v[0] is not None}


def elements(frame):
    """What a reader sees and a test can name: every text where its words
    land, and every button, keyed by kind and label."""
    out = {}
    for t in frame.texts():
        x, y, w, h = t.words()
        if w > 0:
            out.setdefault(("text", t.text), []).append((x, y, w, h))
    for rect, (label, _) in capsules(frame).items():
        out.setdefault(("button", label), []).append(rect)
    for key in out:
        out[key].sort(key=lambda r: (r[1], r[0]))
    return out


def compare_elements(old, new, by):
    problems = []
    for key in sorted(set(old) | set(new), key=lambda k: (k[0], k[1])):
        kind, label = key
        a, b = old.get(key, []), new.get(key, [])
        for ra, rb in zip(a, b):
            dx, dy, dw, dh = rb[0] - ra[0], rb[1] - ra[1], rb[2] - ra[2], rb[3] - ra[3]
            if max(abs(dx), abs(dy)) > by:
                problems.append("%s %r moved %+d,%+d (%d,%d -> %d,%d)" % (kind, label, dx, dy, ra[0], ra[1], rb[0], rb[1]))
            if max(abs(dw), abs(dh)) > by:
                problems.append("%s %r resized %dx%d -> %dx%d" % (kind, label, ra[2], ra[3], rb[2], rb[3]))
        for rb in b[len(a):]:
            problems.append("%s %r is new, at %d,%d" % (kind, label, rb[0], rb[1]))
        for ra in a[len(b):]:
            problems.append("%s %r is missing (was at %d,%d)" % (kind, label, ra[0], ra[1]))
    return problems


# ---- the rules ---------------------------------------------------------


def is_rule(o):
    """A divider: a 1 px line or fill across most of the screen's width or height."""
    x, y, w, h = o.rect
    return o.kind in ("line", "fill") and o.width <= 1 and o.radius == 0 and \
        (h <= 1 and w >= 32 or w <= 1 and h >= 32)


def inside(a, b):
    return a[0] >= b[0] and a[1] >= b[1] and a[0] + a[2] <= b[0] + b[2] and a[1] + a[3] <= b[1] + b[3]


def gap(a, b):
    """Pixels between two boxes (0 when they touch or overlap)."""
    return max(b[0] - (a[0] + a[2]), a[0] - (b[0] + b[2]), b[1] - (a[1] + a[3]), a[1] - (b[1] + b[3]), 0)


def overlaps(a, b):
    return a[0] < b[0] + b[2] and b[0] < a[0] + a[2] and a[1] < b[1] + b[3] and b[1] < a[1] + a[3]


def check_screen(frame, size, run, pushed):
    """Always: every call on the screen."""
    w, h = size
    out = []
    for o in frame.ops:
        x, y, rw, rh = o.rect
        if o.kind != "clip" and (x < 0 or y < 0 or x + rw > w or y + rh > h):
            out.append("off the screen: " + o.line)
    return out


def check_fit(frame, size, run, pushed):
    """Always: every text's box at least its font's line tall, so it draws."""
    out = []
    for o in frame.texts():
        line = run.line_of(o.font)
        if not o.rotated and o.rect[3] < line:
            out.append("text box %d px tall for a %d px line, draws nothing on a device: %s" % (o.rect[3], line, o.line))
    return out


def check_bar(frame, size, run, pushed):
    """Standard 2: the bar starts 8 px from the top and is a line plus 16 px."""
    bar_height = run.line_of(1) + 16
    bar = [t for t in frame.texts() if t.rect[1] < bar_height]
    if not bar:
        return ["no bar: no text in the top %d px" % bar_height]
    first = min(bar, key=lambda t: t.rect[1])
    out = []
    if first.rect[1] != 8:
        out.append("the bar starts at y=%d, not 8: %s" % (first.rect[1], first.line))
    if first.rect[3] != bar_height:
        out.append("the bar is %d px tall, not a line + 16 (%d): %s" % (first.rect[3], bar_height, first.line))
    if pushed:
        if len([o for o in frame.ops if o.kind == "line" and o.width == 3 and o.rect[1] < 8 + bar_height]) < 3:
            out.append("a pushed screen without a back arrow (three 3 px strokes) in the bar")
        if not [o for o in frame.ops if is_rule(o) and o.rect[3] <= 1 and 8 + bar_height <= o.rect[1] <= 16 + bar_height]:
            out.append("a pushed screen without a rule under the bar")
    return out


def check_taps(frame, size, run, pushed):
    """Standard 5 and decision 12: buttons at least the bar's height, 8 px
    apart, and none within 2 px of a rule."""
    bar_height = run.line_of(1) + 16
    out = []
    buttons = sorted(capsules(frame))
    for r in buttons:
        if r[3] < bar_height:
            out.append("a button %d px tall, under the bar's %d: %s" % (r[3], bar_height, r))
    for i, a in enumerate(buttons):
        for b in buttons[i + 1:]:
            if gap(a, b) < 8 and not overlaps(a, b):
                out.append("buttons %d px apart, under 8: %s %s" % (gap(a, b), a, b))
    for r in buttons:
        for rule in (o.rect for o in frame.ops if is_rule(o)):
            if not overlaps(r, rule) and gap(r, rule) < 2:
                out.append("a button within 2 px of a rule: %s %s" % (r, rule))
    return out


def check_actions(frame, size, run, pushed):
    """Standards 2 and 4: at most three actions in the bar, one primary."""
    found = capsules(frame)
    out = []
    if len([r for r in found if r[1] < 8 + run.line_of(1) + 16]) > 3:
        out.append("more than 3 actions in the bar")
    solid = [r for r, (_, s) in found.items() if s]
    if len(solid) > 1:
        out.append("%d solid (primary) buttons, over 1: %s" % (len(solid), solid))
    return out


ALWAYS = {"screen": check_screen, "fit": check_fit}
STANDARDS = {"bar": check_bar, "taps": check_taps, "actions": check_actions}


# ---- scenario files ----------------------------------------------------


class Walk:
    def __init__(self, spec, name, options, events):
        self.spec, self.name, self.events = spec, name, events
        self.board = options.pop("board", spec.board)
        self.fs = options.pop("fs", None)
        self.pushed = options.pop("pushed", None) is not None
        self.sizes = options.pop("sizes").split(",") if "sizes" in options else spec.sizes
        self.skip = set(options.pop("skip", "").split(",")) - {""}
        self.expect = options.pop("expect", None)
        if options:
            raise ValueError("%s: unknown options %s" % (name, sorted(options)))
        unknown = self.skip - set(ALWAYS) - set(STANDARDS)
        if unknown:
            raise ValueError("%s: unknown checks to skip %s" % (name, sorted(unknown)))


class Spec:
    def __init__(self, path):
        self.path, self.app, self.manifest, self.board = path, None, None, "both"
        self.sizes, self.standards, self.walks = list(DEFAULT_SIZES), [], []
        with open(path) as f:
            for number, raw in enumerate(f, 1):
                line = raw.split("  #", 1)[0].strip()  # a comment: a line of its own, or two spaces and #
                if not line or line.startswith("#"):
                    continue
                try:
                    self.read(line)
                except ValueError as e:
                    raise SystemExit("%s:%d: %s" % (path, number, e))
        if not self.app:
            raise SystemExit("%s: no app = line" % path)
        for s in self.standards:
            if s not in STANDARDS:
                raise SystemExit("%s: unknown standard %r (known: %s)" % (path, s, ", ".join(STANDARDS)))

    def read(self, line):
        if line.startswith("walk "):
            head, _, tail = line[5:].partition(" :")
            words = shlex.split(head)
            options = dict(w.partition("=")[::2] for w in words[1:])
            self.walks.append(Walk(self, words[0], options, shlex.split(tail)))
            return
        key, _, value = (part.strip() for part in line.partition("="))
        if key == "app":
            self.app = value
        elif key == "manifest":
            self.manifest = value
        elif key == "board":
            self.board = value
        elif key == "sizes":
            self.sizes = value.split()
        elif key == "standards":
            self.standards = value.split()
        else:
            raise ValueError("unknown setting " + key)


# ---- the check ---------------------------------------------------------


def trace_text(walk, size, result):
    frame = result.frames[-1] if result.frames else Frame([])
    head = ["# %s %s @%s board=%s%s" % (walk.spec.app, walk.name, size, walk.board, " fs=" + walk.fs if walk.fs else ""),
            "# events: " + " ".join(walk.events),
            "# frames: %d, then %s" % (len(result.frames), result.ending()),
            "# line heights %s" % " ".join(map(str, result.line_heights))]
    tail = ["# drawn after the last refresh:"] + [o.line for o in result.pending] if result.pending else []
    return "\n".join(head + [o.line for o in frame.ops] + tail) + "\n"


def frame_of(text):
    return Frame([Op(l) for l in text.splitlines() if l and not l.startswith("#")])


def ending_of(text):
    return text.splitlines()[2] if text.count("\n") > 2 else ""


def cut(lines):
    if len(lines) <= DIFF_LINES:
        return lines
    return lines[:DIFF_LINES] + ["... %d more" % (len(lines) - DIFF_LINES)]


def check_walk(walk, size, update, show, standards=None):
    w, h = (int(v) for v in size.split("x"))
    tmp = tempfile.mkdtemp(prefix="screen-test-")
    env = {"DESSEN_SCREEN": size, "DESSEN_BOARD": walk.board, "DESSEN_SNAPSHOT": os.path.join(tmp, "screen.png")}
    if walk.spec.manifest:
        env["DESSEN_MANIFEST"] = os.path.join(ROOT, walk.spec.manifest)
    try:
        env.update(storage(walk.fs, tmp))
        result = drive(walk.spec.app, env, walk.events, (w, h))
        again = drive(walk.spec.app, env, walk.events, (w, h)) if walk.fs is None and standards is None else None
    except LookupError as e:
        return ["could not walk: %s" % e], []
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    problems, notes = [], []
    if not result.frames or result.png is None:
        problems.append("nothing was drawn and refreshed (%s)" % result.ending())
    elif walk.expect and not any(walk.expect in t.text for t in result.frames[-1].texts()):
        # The walk did not reach the screen it is for: a tap that missed.
        problems.append("did not reach its screen: no text with %r on it (it shows %s)"
                        % (walk.expect, sorted({t.text for t in result.frames[-1].texts()})))
    if again is not None and (again.png != result.png or trace_text(walk, size, again) != trace_text(walk, size, result)):
        problems.append("the same walk drew a different picture the second time (standard 9)")
    for index, frame in enumerate(result.frames):
        for name, check in ALWAYS.items():
            if name not in walk.skip:
                problems += ["frame %d: [%s] %s" % (index + 1, name, p) for p in check(frame, (w, h), result, walk.pushed)]
    if result.frames:
        for name in walk.spec.standards if standards is None else standards:
            if name not in walk.skip:
                problems += ["[%s] %s" % (name, p) for p in STANDARDS[name](result.frames[-1], (w, h), result, walk.pushed)]
    problems = dedupe(problems)
    if standards is not None or result.png is None:
        return problems, notes

    text = trace_text(walk, size, result)
    base = os.path.join(GOLDEN, walk.spec.app, "%s@%s" % (walk.name, size))
    rel = os.path.relpath(base, ROOT)
    if show:
        sys.stdout.write(text)
    os.makedirs(os.path.dirname(base), exist_ok=True)
    if not os.path.exists(base + ".png"):
        write(base + ".png", result.png)
        write(base + ".trace", text)
        return problems + ["new golden written, look at it and run again: %s.png" % rel], notes
    old_png = open(base + ".png", "rb").read()
    old_text = open(base + ".trace").read() if os.path.exists(base + ".trace") else None

    differs = []
    if old_png != result.png:
        old_pic, new_pic = read_png(old_png), read_png(result.png)
        count, box, bad = compare_pictures(old_pic, new_pic, TOLERANCE)
        if count < 0:
            differs.append("the picture changed size")
        elif count:
            diff = os.path.join(DIFFS, walk.spec.app, "%s@%s.png" % (walk.name, size))
            write_diff(diff, old_pic, new_pic, bad)
            differs.append("%d pixels of ink moved more than %d px, within %d,%d %dx%d; see %s"
                           % (count, TOLERANCE, *box, os.path.relpath(diff, ROOT)))
        else:
            notes.append("the picture moved within %d px: %s.png" % (TOLERANCE, rel))
    if old_text is not None and old_text != text:
        moved = compare_elements(elements(frame_of(old_text)), elements(frame_of(text)), TOLERANCE)
        if moved:
            differs.append("elements changed:\n          " + "\n          ".join(cut(moved)))
        if ending_of(old_text) != ending_of(text):
            differs.append("it ends differently: %s, was %s" % (ending_of(text)[2:], ending_of(old_text)[2:]))
        if not moved and not differs:
            notes.append("the draw calls changed, the screen did not: %s.trace" % rel)
        if differs and not update:
            calls = list(difflib.unified_diff(old_text.splitlines(), text.splitlines(), "golden", "now", lineterm="", n=0))
            differs.append("the draw calls (diff):\n          " + "\n          ".join(cut(calls)))

    if update and (old_png != result.png or old_text != text):
        write(base + ".png", result.png)
        write(base + ".trace", text)
        notes.append("golden updated: " + rel)
    elif differs:
        problems.append("differs from %s:\n        - %s" % (rel, "\n        - ".join(differs)))
    elif old_text is None:
        write(base + ".trace", text)
    return problems, notes


def write(path, data):
    with open(path, "wb" if isinstance(data, bytes) else "w") as f:
        f.write(data)


def dedupe(items):
    seen, out = set(), []
    for item in items:
        key = re.sub(r"^frame \d+: ", "", item)
        if key not in seen:
            seen.add(key)
            out.append(item)
    return out


def main(argv):
    update = "--update" in argv or os.environ.get("UPDATE_GOLDENS") == "1"
    show, report = "--show" in argv, "--standards" in argv
    only = None
    if "--only" in argv:
        only = argv[argv.index("--only") + 1]
        argv = argv[:argv.index("--only")] + argv[argv.index("--only") + 2:]
    files = [a for a in argv if not a.startswith("--")]
    if not files:
        sys.exit(__doc__)
    if not os.access(RUNNER, os.X_OK):
        sys.exit("no %s: build it first (make build/dessen)" % os.path.relpath(RUNNER, ROOT))
    failed = checked = 0
    for path in files:
        spec = Spec(path)
        broken = {}
        for walk in spec.walks:
            if only and only not in (spec.app, spec.app + "/" + walk.name):
                continue
            for size in walk.sizes:
                checked += 1
                label = "%s/%s@%s" % (spec.app, walk.name, size)
                if report:
                    problems, _ = check_walk(walk, size, False, False, standards=list(STANDARDS))
                    for p in problems:
                        broken.setdefault(re.sub(r"^frame \d+: ", "", re.sub(r"[:(].*", "", p)), []).append(label)
                    continue
                problems, notes = check_walk(walk, size, update, show)
                for n in notes:
                    print("  note  %s: %s" % (label, n))
                if problems:
                    failed += 1
                    print("  FAIL  %s" % label)
                    for p in problems:
                        print("        " + p)
        if report:
            print("%s (declares: %s)" % (spec.app, " ".join(spec.standards) or "none"))
            for rule, where in sorted(broken.items()):
                print("    %-66s %3d walks, e.g. %s" % (rule, len(where), where[0]))
            if not broken:
                print("    meets every standard")
    if report:
        return 0
    print("screen tests: %d walks, %d failed (tolerance %d px)" % (checked, failed, TOLERANCE))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
