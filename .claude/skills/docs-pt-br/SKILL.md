---
name: docs-pt-br
description: Writes or revises smart-alerts text in controlled Brazilian Portuguese — README.md, wiki/pt-BR pages, and the pt-BR text of alert messages. Use for any pt-BR writing in this repository.
---

# docs-pt-br

pt-BR is the first language of smart-alerts. The author writes it by hand, with the EN text in the
same change.

If the `ptbr-writer` skill is available, apply it, and stop reading here. Use level `estrito`
for alert messages and runbooks, and level `descritivo` for README and wiki pages. The rules below
are the fallback when that skill is not installed.

## Rules

1. **Explicit subject.** Who acts appears in the sentence. Avoid the `-se` passive and `o mesmo`.
2. **One statement per sentence.** Cut connecting gerunds and clauses that hide a second action.
3. **Fixed modality.** `deve` = obligation. `pode` = permission or capability. A probability is a
   number or a range, never a vague modal.
4. **Full verb.** `realizar a validação` → `validar`.
5. **Frozen terms.** One concept, one term. Use the terms of this repository: `alerta`, `fonte`,
   `harness`, `estágio`, `UF`, `severidade`. Code identifiers go in backticks, not translated.
6. **Numbers, dates, units.** Decimal comma. ISO 8601 dates. Unit after the number (`50 mm`).
7. **Fact apart from analysis.** Each sentence is a fact with a source or the author's reading,
   and says which one.
8. **Mark what was not checked.** A claim nobody checked gets `⚠`. Never remove a `⚠` without
   doing the check.

## Alert messages

An alert message is read once, maybe at night, maybe by a frightened person.

- Lead with the hazard and the place: `Risco de inundação em Porto Alegre (RS).`
- Use the severity words of the source. Do not upgrade or downgrade them.
- Give one action per sentence, in the imperative, only when the source gives it.
- Never invent an action, a time or an area that the source does not state.

## Pages

- `README.md` (pt-BR) and `README.en.md` mirror each other: same sections, same facts. A change
  to one changes the other in the same PR.
- A wiki page lives in `wiki/pt-BR/<Page>-pt-BR.md`. Its English twin is
  `wiki/en/<Page>-en.md`.

## Output

1. The final text.
2. A table: rule, original passage, rewrite (only when revising).
3. The list of `⚠` claims.
