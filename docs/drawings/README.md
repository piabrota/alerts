# Drawings

## `wave.svg`

A wave and an abstract bell. `wave.py` writes the drawing in two formats from one geometry:

| File | Command | What it is |
|---|---|---|
| `wave.svg` | `python3 docs/drawings/wave.py > docs/drawings/wave.svg` | The README drawing |
| `wave.desmos.json` | `python3 docs/drawings/wave.py desmos > docs/drawings/wave.desmos.json` | The same drawing as a Desmos graph state |

**The wave is a Fibonacci spiral.** It has eight quarter-circle arcs. The radii are
5 × (21, 13, 8, 5, 3, 2, 1, 1). Each arc fills one square of side 5 × Fₙ:
`python3 docs/drawings/wave.py squares` draws the squares.

**The bell** is a half circle (radius 14), a rim, a clapper dot and two sound arcs (radius 18).

### Open the Desmos file

The file is a Desmos graph state (`version` 11), with parametric expressions only. Desmos uses
y up, so each y value is `180 - y` of the SVG.

1. Open `https://www.desmos.com/calculator`.
2. Open the browser console.
3. Run `Calc.setState(<the content of wave.desmos.json>)`.

⚠ Not checked: the Desmos page could not be opened when this file was made. The check was a
local one: `docs/screenshots/wave/desmos-overlay.png` draws each Desmos expression (orange) over
the SVG (blue), and the two match. Desmos is a free service but not FOSS: the file is data, and
smart-alerts does not depend on Desmos.

### Credits

The drawing is original. It joins two old ideas, and other artists joined them before:

- The Fibonacci sequence: Leonardo of Pisa (Fibonacci), *Liber Abaci*, 1202. The spiral of
  quarter circles in Fibonacci squares is a common construction.
- The wave as a spiral: Katsushika Hokusai, *The Great Wave off Kanagawa* (c. 1831), public
  domain. People often draw a Fibonacci spiral over it.
- Similar works by other artists, found on 2026-10-26 (not copied, not checked in detail ⚠):
  "Hokusai Meets Fibonacci" by cartogram and "The Great Fibonacci Wave" by Gypsy Queen, both on
  [TeePublic](https://www.teepublic.com/fibonacci-wave-merchandise).
