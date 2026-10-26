---
name: figure
description: Add, change or explain a diagram in this repo's Markdown (README.md, README.pt-BR.md, docs, wiki): a Mermaid fence or an Archify picture. Use when asked for a diagram, a flowchart or "a picture of" something, or to update one after its prose changed.
---

# figure

This verb is the `figure` skill of ptbr-writer, adapted to smart-alerts.

A diagram is a ```` ```mermaid ```` fence in the page it illustrates (GitHub draws it there), or a
PNG drawn with Archify. Every diagram has a header and a reading card.

## Steps

1. **Read the prose the figure sits beside.** Draw only what it says. The text is the source of
   truth: a figure that disagrees with it is a bug in the figure. `README.md` and
   `README.pt-BR.md` say the same, so their figures say the same.
2. **Write the fence.** The first two lines, which Mermaid ignores:

   ```
   %% figure: <id>          lowercase words joined by '-', unique in the repo
   %% title: <question>     the question the figure answers
   ```

   Then the diagram, at most about 15 nodes. Split it rather than grow it.
3. **Write the reading card** right after the fence or the PNG, in the page's language:

   ```
   <details open>
   <summary>How to read this figure</summary>     (pt-BR: Como ler esta figura)

   **Takeaway.** one plain sentence, what to remember        (Em uma frase.)
   **How to read.** direction, what shapes and arrows mean   (Como ler.)
   **Not shown.** what was left out on purpose               (Fora da figura.)
   **Evidence.** file, paragraph or command, with ⚠ if unchecked  (Evidência.)

   </details>
   ```

4. **Check** that every node and label traces to a file you read. Never remove a ⚠ without doing
   the check.

## Visual grammar

- Rectangle: a thing or a step. Diamond: a test, written as a question. Every test labels both its
  *yes* and its *no* edge.
- Solid arrow: makes or leads to the next thing. Dashed arrow: checks or compares against. Label
  every arrow that is not obvious from its two ends.
- One flow direction per figure. Groups (`subgraph`) for stages.
- No `classDef`, `style` or `%%{init}%%`. A colour never carries meaning on its own.

## A picture drawn with Archify

For a flow that a Mermaid fence draws badly, draw a picture.

1. Write the source as `docs/archify/<slug>.json`. The schema is in the Archify documentation, in
   the pinned submodule `vendor/archify` (run `just figure-init` once).
2. Write the English twin `docs/archify/en/<slug>.json`: the same JSON with `meta.locale` `en` and
   only the words translated. If a translated label breaks the layout, shorten the label; never
   move a box.
3. Run `just figure`. It writes `docs/img/<slug>.png`, `docs/img/en/<slug>.png` and
   `docs/archify/figures.lock`. A crop height goes in `docs/archify/crop.tsv`.
4. `README.pt-BR.md` embeds `docs/img/<slug>.png`. `README.md` embeds `docs/img/en/<slug>.png`.
   Put the same reading card under each.
5. Look at the PNG at full size and at 800 px wide: every label must read. Commit the JSON, the
   lock and the PNG, never `build/`. `just figure-check` runs in `just check`.

The Desmos drawing (`docs/drawings/`) is not a diagram: `just wave` keeps drawing it.
