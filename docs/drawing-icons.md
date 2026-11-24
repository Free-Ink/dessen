# E-paper graphics for mini-games — a guide for agents

How to draw the graphics of a small game on a dessen device: the pieces, the
board they sit on, the marks that show state. It is written for an agent that
has to produce good-looking, legible, cheap-to-draw pictures in code, without
an artist and without bitmaps.

Everything here was learned building Book Nook
([`apps/xpui/nook/src/paint.rs`](../apps/xpui/nook/src/paint.rs)), which is the
worked example at the end. The rules are general; the plants and books are
only where they were found.

The style in one sentence: **solid ink shapes built from a few rounded and
pointed primitives, layered back to front with a thin paper edge between
them, one grey for material, and tiny paper details cut into the ink.**

## 1. The medium

- **One bit.** Ink (`DSN_BLACK`), paper (`DSN_WHITE`), and two greys
  (`DSN_LIGHT_GRAY`, `DSN_DARK_GRAY`) that are really dither patterns. No
  anti-aliasing, no alpha, no colour. A reference photo's colours and soft
  shadows have to become silhouette and pattern.
- **Small.** A game piece is 25 to 70 pixels across. A detail under 2 pixels
  disappears and a gap under 2 pixels closes.
- **Slow to change.** The panel repaints in a visible flash, and what changed
  can leave a ghost. Nothing animates. The picture is judged still.
- **Redrawn whole.** The screen is repainted after every input, so a graphic
  must come out identical every time it is drawn in the same state.
- **Drawn, not stored.** Graphics are code. They scale to every panel (480 to
  1448 pixels wide) and cost no asset bytes, but every draw call is a system
  call and the whole app has 64 KB.

## 2. Decide what each thing is for

Before drawing anything, sort every graphic in the game into one of four
roles. The role sets how loud it is.

| Role | What it is | How it is drawn |
|---|---|---|
| **Pieces** | what the player places, moves or collects | the loudest thing on screen: solid ink masses with paper edges |
| **Fixtures** | what the puzzle gives and the player cannot change | as rich as pieces, but visibly a different kind of object |
| **Structure** | the board, grid, table, container | quiet: thin ink lines, grey for material, paper ground |
| **Marks** | state laid over the rest: selected, wrong, done, counts | small, high contrast, the same everywhere in the app |

Rules that follow from the roles:

- **A piece must never look like a fixture or like decoration.** If the player
  can place plants, nothing else on the board may look like a plant. (Book
  Nook once had a decorative vase beside the books; it read as a placed
  piece and had to go.)
- **Empty is empty.** A free cell is paper, perhaps a shadow. No ornament.
- **Structure recedes.** If the grid is the first thing you see, it is too
  heavy.
- **Sets differ by silhouette.** Two kinds of piece that differ only in a
  detail are the same piece at arm's length. Change the outline.
- **Sides differ by tone.** Two players, or two states of one piece, are
  solid ink against outlined paper. Grey is a third tone, for "off" or
  "wrong", never the only difference between two things that matter.

## 3. The primitives

| Shape | Call | Use it for |
|---|---|---|
| Rectangle | `Renderer::fill_rect(rect, ink)` | boards, bands, bars, erasing with paper |
| Grey rectangle | `Renderer::fill_rect_dither(rect, light)` | material and shadow |
| Rounded box, circle | `dessen::dsn_fill_round(x, y, w, h, color, radius, corners)` | tokens, pips, capsules, anything soft |
| Rounded outline | `dessen::dsn_stroke(x, y, w, h, color, width, radius, corners)` | the ink edge round a pale shape |
| Triangle | `dessen::dsn_triangle(ax, ay, bx, by, cx, cy, color)` | anything pointed or tapered |
| Line with a width | `dessen::dsn_line(x0, y0, x1, y1, width, color)` | strokes: ticks, crosses, arrows, stems |
| Text in a box | `dessen::dsn_text_box(…)` | every number and letter: never draw glyphs |

`Renderer` is xpui's; the `dessen::dsn_*` calls are the SDK's own and use the
same coordinates, so mix them freely (in C they are the same calls without
the prefix). An app that draws this way still needs no `unsafe`.

Four shapes cover nearly everything:

```rust
// A circle: a rounded box whose radius is half its side.
fn disc(across: i32, down: i32, radius: i32, color: i32) {
    let wide = 2 * radius + 1;
    dessen::dsn_fill_round(across - radius, down - radius, wide, wide, color, radius, dessen::DSN_CORNERS_ALL);
}
```

- **Disc**: as above.
- **Capsule**: a rounded box with a radius of half its short side. Columns,
  buttons, limbs, pills.
- **Kite**: two triangles sharing the line from base to tip, widest part of
  the way along. Leaves, flames, blades, petals, arrowheads.
- **Taper**: two triangles making a trapezoid. Pots, shades, roofs, hulls.

The kite needs the length of its axis, and there is no square root. "The
longer side plus three eighths of the shorter" is near enough:

```rust
let (long, short) = (across.abs().max(up), across.abs().min(up));
let length = (long + short * 3 / 8).max(1);
let (side_x, side_y) = (up * half / length, across * half / length);
```

## 4. The rules

### Everything is a fraction of the box

Take the box you are given and derive every measurement from it. The only
pixel constants are floors and hairlines:

```rust
let half = (width / 12).max(2);     // never under 2 px
let tall = height * 2 / 5;
```

Integer maths only, and multiply before dividing (`width * 3 / 10`, not
`width / 10 * 3`). The graphic must hold at the smallest box the game will
give it, not only the comfortable one.

### Silhouette first

Squint at the reference. The shape that survives is what you draw, in solid
ink. If the black blob alone does not say what it is, no detail will rescue
it. Lines alone read as stick figures: give every part area.

### The paper edge is the whole trick

Where parts overlap, draw each part **twice**: once in paper, a pixel or two
larger, then in ink at its real size. Work back to front. Each part then
stands off from whatever is behind it with a clean pale outline, and a pile
of black shapes reads as layers instead of one blob.

```rust
for (across, down) in parts {           // back to front
    disc(x, y, radius + 1, PAPER);      // the edge
    disc(x, y, radius, INK);            // the part
}
```

The same trick puts a piece on a busy background: a paper halo first, then
the piece.

### One grey, for material and shadow, inside ink

Grey is a dither, so it has a ragged edge and turns to noise in anything
thin. Use it for one material per object and for shadow, always bounded by
ink, never for outlines, never in shapes under about 4 pixels, and never as
a large field (it shimmers and ghosts). The two greys are two parities of the
same checkerboard: side by side they look alike, so do not rely on telling
them apart.

### Details are paper cut into ink

A vein, a seam, a glint, a pip, a speckle: one-pixel paper marks on ink (or
ink on paper), added last and only when the part is big enough to hold them
(`if half >= 3`). Two or three kinds of detail per graphic is plenty.

### Stroke weights

- **3 px** for a mark that stands alone: tick, cross, arrow, a symbol in a bar.
- **2 px** for the edge of a pale shape, and for a mark inside a cell.
- **1 px** for dividers and for detail inside a shape. Never for a symbol: a
  one-pixel glyph beside bold type looks faint.

### Ground everything

Give an object a floor and build up from it. Draw a container first and its
contents over the rim. Fill most of the box: an object using half its cell
looks lost.

### Depth costs two rectangles

A strip of light grey under an overhanging edge is its shadow; a 1 px paper
line just inside an ink edge is light catching it. Pick one light direction
(top left) and keep it. Without these a board is a flat grid; with them it is
a thing.

### Thick holds, thin divides

The outside of a board, tray or case is thick; the lines between its cells
are thin. That alone says "one object with compartments" rather than "a
table of squares".

### Vary by position, never by chance

For variety (which of four designs, how tall each one), run a small generator
from a seed that depends only on *where* the graphic is, never on time or the
device's random source, or the picture changes under the player at every
repaint. Stir the seed a round or two first: neighbouring cells' seeds differ
by a constant, and unstirred they come out the same.

```rust
fn next(seed: &mut u32) -> i32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed >> 8) as i32
}
```

### A family shares its parts

A set looks like a set when its members share a base, an ink weight and the
same two or three helpers. Write the shared parts first; each member is then
a short table of positions. This is also what keeps the code small.

## 5. Showing state

State belongs on the thing, in the same visual language across the app:

| State | Treatment |
|---|---|
| To do | paper with a 2 px ink edge |
| Done, right, on | solid ink, content in paper |
| Wrong, over, disabled | grey with an ink edge |
| Breaks a rule | a small ink badge with a paper `!`, on a paper halo, at the top right of the offender |
| Selected | a ring inset from the cell's edge, or the cell inverted; never a grey wash under ink art |
| Crossed out, empty on purpose | a small 2 px cross, centred |

Numbers and letters are always the firmware font in a box, never drawn.
Inverted text (`DSN_TEXT_INVERTED`) gives paper digits on an ink plaque.

## 6. Kind to the panel, cheap to draw

- **Change as little as possible per move.** A tap should alter one cell and
  a counter, not restyle the board. Large areas that flip between ink and
  paper flash and ghost.
- **Keep ink in shapes and type.** No large black panels, no inverted
  screens.
- **Count draw calls.** Each one crosses into the host. Book Nook's whole
  screen is about 500 calls empty and 800 full; one plant is 30 to 70. Aim
  for tens per piece and under a thousand per screen.
- **Prefer one big primitive to a loop of small ones.** Two triangles, not a
  row of one-pixel rectangles per scanline. A rounded box, not a stack of
  rectangles.
- **Tables, not code.** A graphic is a short list of positions in tenths or
  twentieths of its box and a loop over it.
- **No `format!`, no floats, no bitmaps** unless the picture truly cannot be
  drawn (a book cover is a bitmap; a game piece is not).
- **Measure.** The desktop runner prints instructions per run, and
  `make xpui-sizes` / `make xpui-peaks` the image size and the heap and stack
  peaks. Check them after adding art.

## 7. Starting points for common pieces

Constructions, not finished art. Only the ones in the worked example have
been looked at on a device; build any other, then look at it (section 9).

| Piece | Construction |
|---|---|
| Counter, stone, coin | a disc; the other side is a paper disc with a 2 px edge; a king or a value is a smaller paper disc or a font digit inside |
| Die | a rounded box with a 2 px edge, pips as discs on a 3 × 3 grid of positions |
| Playing card | a rounded box with a 2 px edge, rank in the font at the corner, one suit shape in the middle |
| Heart | two discs side by side and a triangle under them |
| Spade | a heart upside down on a small taper |
| Diamond | a kite as wide as it is tall, or two triangles |
| Club | three discs and a small taper |
| Star | a triangle up and a triangle down for six points; five need a table of points |
| Arrow | a 3 px shaft and two 3 px strokes for the head |
| Flag, pennant | a 2 px pole and a triangle |
| Key, tool, handle | a disc with a paper hole, a capsule for the shaft, small rectangles for teeth |
| Gem | a taper over an inverted taper, one paper facet line |
| Bomb, ball | a disc, a paper glint, a 2 px fuse |
| Tree, bush | discs on a capsule trunk, each on a paper edge |
| House, block | a rectangle under a triangle, a paper door |
| Wall, crate, tile | a board (ink edges, grey fill, a paper highlight) |
| Character | a disc for the head on a capsule or taper for the body; no face under 40 px |

## 8. Working from a reference

1. Sort it into a role (section 2) and decide how loud it may be.
2. Name the three or four shapes it is made of.
3. Note the proportions as fractions of the box.
4. Decide what the one grey is for, if anything.
5. Write positions as a table, back to front, and loop over it.
6. Look at it, then fix what your eye lands on first. Repeat.

## 9. Look at it

You cannot judge a graphic from its code or from the desktop runner's trace.
Render it and look, every round:

- The crosspoint simulator draws with the real firmware fonts and dithers.
  Drive it with `CROSSPOINT_SIM_INPUT_SCRIPT` (`TAP:x,y`, then a later
  `SHOT:path.bmp`, then `QUIT`), convert with `sips -s format png`, and read
  the image. Enlarge the part you are judging.
- Put several instances side by side, with different seeds, at the smallest
  and the largest size the game uses, and on the board they will live on.
- Ask of each: is it recognisable in a second? Could it be mistaken for
  another piece, or for something that is not a piece? What is the first
  thing that looks wrong?

## 10. Mistakes already made

- **Lines for leaves** read as sticks. Give parts area.
- **Drawing what a thing does** (rays under a lamp) read as legs. Draw what
  it is.
- **Big bumps** on an edge read as berries; one-pixel bumps, closely spaced,
  read as ridges.
- **A grey pattern across a small shape** looked like a window grille.
- **An object much smaller than its base or its cell** looks like a picture
  of the base.
- **Decoration that looks like a piece.**
- **Neighbouring cells all drawing the same variant**, from unstirred seeds.
- **A one-pixel icon from a library** beside bold type looked faint; three
  3 px strokes were better.
- **A text box shorter than the font's line height draws nothing.** Size it
  from `dsn_line_height`.

## 11. Checklist

- [ ] Every graphic has a role, and pieces are the loudest thing on screen.
- [ ] No two pieces share a silhouette; nothing else looks like a piece.
- [ ] Every size comes from the box, with a floor in pixels.
- [ ] Overlapping parts are drawn paper-then-ink, back to front.
- [ ] Grey appears for material or shadow only, inside ink, in nothing thin.
- [ ] Marks are 3 px strokes; state uses the outlined / solid / grey / badge
      language.
- [ ] The same state draws the same picture every time.
- [ ] One move changes a small part of the screen.
- [ ] Tens of calls per piece, under a thousand per screen; sizes measured.
- [ ] Looked at in the simulator, at the smallest size, next to its siblings.

## Worked example: Book Nook

How the rules land in one game. The code is
[`apps/xpui/nook/src/paint.rs`](../apps/xpui/nook/src/paint.rs); read it for
the exact proportions.

- **Roles.** Plants are the pieces, books are the fixtures, the bookcase is
  the structure, and the number plaques, `!` badges and crosses are the
  marks.
- **Pieces: four plants.** One shared pot (a rounded box, paper with a 2 px
  edge, a grey band at the foot as the only grey, a few speckles) and four
  crowns with four different outlines: a burst of thin kites, a few broad
  kites, three knobbed capsules, a rosette of discs. Each is drawn over the
  pot's rim, parts back to front on paper edges. Which plant appears depends
  on the cubby, so it never changes on repaint.
- **Fixtures: rows of books.** Rectangles in three bindings (solid ink, paper
  with an edge, grey with an edge), never two alike side by side, with heights
  and widths from the seed, ending in a leaning book, a flat stack or nothing.
  Titles are bars, not letters. Nothing about them can be mistaken for a
  plant.
- **Structure: the bookcase.** Every board is one function: ink edges, grey
  between, a paper highlight on the lit edge. Thick sides, top, crown and
  plinth; thin uprights between cubbies; a strip of grey under each shelf for
  depth; a paper back so ink objects have a clean ground.
- **Marks.** Number plaques go outlined, solid, grey as a row fills; a plant
  touching another gets the `!` badge; a crossed-out cubby gets a small cross.
- **Cost.** About 500 draw calls for the empty case and 30 to 70 per plant, in
  an app that asks for under 45 KB.
