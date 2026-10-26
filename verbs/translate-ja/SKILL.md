---
name: translate-ja
description: Translates a smart-alerts English wiki page into Easy Japanese (やさしい日本語) (wiki/ja). Use when a ja page is missing or `just docs-stale` reports it stale, or when the user asks for Japanese documentation.
---

# translate-ja

The ja pages are translations of the EN pages into やさしい日本語 (Easy Japanese): plain Japanese
for readers who are not fluent. Never write a ja page from the pt-BR text or from scratch, and
never add a fact that the EN page does not state.

## Process

1. Read the source page `wiki/en/<Page>-en.md`.
2. Translate it with the rules YN-1 to YN-10.
3. Write `wiki/ja/<Page>-ja.md`. The first line is the source line, the second is the review
   banner:

   ```
   <!-- source: wiki/en/<Page>-en.md sha: <output of git hash-object wiki/en/<Page>-en.md> reviewed: no -->
   > ⚠ この ページは 英語の ページから 翻訳（ほんやく）しました。日本語を 話す 人が まだ チェックしていません。
   ```

4. Run `just docs-stale`. The page must not be listed.
5. Give the user a literal back-translation into English, so they can check the meaning.

Only a native reader sets `reviewed: yes` and removes the banner. Never do it yourself.

## Rules

- **YN-1 One idea per sentence.** A sentence has at most 40 characters (repository convention).
- **YN-2 です／ます form.** Do not use 尊敬語 or 謙譲語.
- **YN-3 Common words.** Prefer a common word to a technical Sino-Japanese compound. When a
  technical term is necessary, give its reading in parentheses at its first use: `浸水（しんすい）`.
- **YN-4 Explicit subject** when the subject is not obvious from the previous sentence.
- **YN-5 No double negatives** and no vague endings (`〜かもしれません`) without a number or a
  condition.
- **YN-6 Katakana loanwords** only when they are common (`データ`, `サーバー`). Otherwise, use a
  plain Japanese word.
- **YN-7 Fixed terms.** `警報（けいほう）` (alert), `データの 出どころ` (source), `harness` (keep
  in Latin script), `段階（だんかい）` (stage), `州（しゅう）` (UF). Code identifiers go in backticks,
  not translated.
- **YN-8 Numbers, dates, units.** Arabic numerals. Dates as `2026年10月22日`. Explicit unit
  (`50 mm`).
- **YN-9 Spaces between phrases (分かち書き)** to help readers find word borders, as in the
  banner above.
- **YN-10 Lists for parallel items.** A paragraph has at most 4 sentences.

## Output

1. The path of the page you wrote.
2. The back-translation into English.
3. The terms you were not sure about.
