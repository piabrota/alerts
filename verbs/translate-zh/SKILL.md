---
name: translate-zh
description: Translates a alerts English wiki page into controlled Simplified Chinese (wiki/zh). Use when a zh page is missing or `just docs-stale` reports it stale, or when the user asks for Chinese documentation.
---

# translate-zh

The zh pages are translations of the EN pages. Never write a zh page from the pt-BR text or from
scratch, and never add a fact that the EN page does not state.

## Process

1. Read the source page `wiki/en/<Page>-en.md`.
2. Translate it with the rules ZC-1 to ZC-10.
3. Write `wiki/zh/<Page>-zh.md`. The first line is the source line, the second is the review
   banner:

   ```
   <!-- source: wiki/en/<Page>-en.md sha: <output of git hash-object wiki/en/<Page>-en.md> reviewed: no -->
   > ⚠ 本页由英文页面翻译。母语者尚未审校。
   ```

4. Run `just docs-stale`. The page must not be listed.
5. Give the user a literal back-translation into English, so they can check the meaning.

Only a native reader sets `reviewed: yes` and removes the banner. Never do it yourself.

## Rules

- **ZC-1 One action per sentence.** A descriptive sentence has at most 40 Chinese characters. An
  instruction has at most 30.
- **ZC-2 Explicit subject, active voice.** Avoid `被` without an agent.
- **ZC-3 One term, one concept.** `警报` (alert), `数据源` (source), `harness` (keep in Latin
  script), `阶段` (stage), `州` (UF), `严重程度` (severity). Code identifiers go in backticks, not
  translated.
- **ZC-4 Full verb.** `进行测试` → `测试`.
- **ZC-5 Fixed modality.** `必须` = obligation. `应` = recommendation. `可以` = permission or
  capability. Do not use `可能` without a number or a condition.
- **ZC-6 Short 的 chains.** At most 2 `的` per noun phrase.
- **ZC-7 No 成语, slang, irony or metaphor.**
- **ZC-8 Numbers and units.** Arabic numerals. Explicit unit (`50 mm`, `5 秒`).
- **ZC-9 Lists for parallel items.** A paragraph has at most 4 sentences.
- **ZC-10 Typography.** Mainland Simplified Chinese. Full-width punctuation (`，。：；（）`). A space
  between a Chinese character and a Latin word or a number.

## Output

1. The path of the page you wrote.
2. The back-translation into English.
3. The terms you were not sure about.
