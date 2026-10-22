---
name: docs-en
description: Writes or revises smart-alerts English text in a simplified technical English modeled on ASD-STE100 — README.en.md and wiki/en pages. Use for any English documentation in this repository. The English pages are the source for the zh and ja translations.
---

# docs-en

EN is the second hand-written language of smart-alerts. It mirrors pt-BR, and it is the source
for the zh and ja pages, so it must translate cleanly.

## Rules

1. **Short sentences.** A procedure sentence has at most 20 words. A descriptive sentence has at
   most 25 words.
2. **One instruction per sentence.** Write procedures in the imperative: `Run just check.`
3. **Active voice, simple tenses.** Use the present tense when possible.
4. **One word, one meaning.** Use the terms of this repository: `alert`, `source`, `harness`,
   `stage`, `UF`, `severity`. Do not use synonyms for variety. Code identifiers go in backticks.
5. **No phrasal verbs or idioms** when a single verb exists: `set up` → `configure`.
6. **Keep articles** (`a`, `an`, `the`). They make the text easier to translate.
7. **Fixed modality.** `must` = obligation. `can` = capability. A probability is a number.
8. **Paragraphs** have at most 6 sentences. Use a list for parallel items.
9. **Mark what was not checked.** A claim nobody checked gets `⚠`. Never remove a `⚠` without
   doing the check.

## Pages

- `README.en.md` mirrors `README.md` (pt-BR): same sections, same facts.
- A wiki page lives in `wiki/en/<Page>-en.md`. Its pt-BR twin is `wiki/pt-BR/<Page>-pt-BR.md`.
- When an EN wiki page changes, its zh and ja pages become stale. Run `just docs-stale` and tell
  the user which pages need the `docs-zh` or `docs-ja` skill.

## Output

1. The final text.
2. A table: rule, original passage, rewrite (only when revising).
3. The list of `⚠` claims.
