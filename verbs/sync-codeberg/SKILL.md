---
name: sync-codeberg
description: Publica a cópia limpa do smart-alerts no Codeberg (piabrota/ialerts), sem rastro de IA, com `just sync-codeberg`. Use quando pedirem para sincronizar, espelhar ou fazer push para o Codeberg, ou depois de um merge que deve chegar ao espelho.
---

# sync-codeberg

O espelho `codeberg.org/piabrota/ialerts` recebe só a cópia limpa do código. O comando
`just sync-codeberg` (Rust, `scripts/export-codeberg/src/bin/sync-codeberg.rs`) faz tudo:

1. Lê o branch `main` do Codeberg. Se o branch não existe, começa um histórico novo.
2. Troca a árvore pela cópia limpa do `HEAD` (o mesmo trabalho do `just export-codeberg`).
3. Faz um commit com uma mensagem simples, com o autor como autor e como committer. Os hooks ficam
   desligados e a assinatura fica desligada, a não ser com `--sign`.
4. Confere a árvore, o autor e a mensagem. Qualquer rastro de IA para o comando.
5. Faz o push para `main`, sem `--force`.

O histórico deste repositório nunca vai para o Codeberg: o commit novo tem como pai só o último
commit do Codeberg.

## Como usar

1. Rode o comando a partir do `main` atualizado, com a árvore limpa.
2. Primeiro, rode um ensaio e confira o commit no diretório que o comando mostra:

   ```bash
   just sync-codeberg --author "M.Hoffmann <hoffmann@poli.ufrj.br>" --dry-run
   ```

3. Depois, rode sem `--dry-run`. O push usa as credenciais do git da máquina (HTTPS ou SSH, com
   `--remote git@codeberg.org:piabrota/ialerts.git`).

Sem `--author`, o comando usa `user.name` e `user.email` deste repositório. Ele recusa um autor
que nomeia uma ferramenta de IA: nesse caso, passe `--author`.

## Regras

- Nunca faça push deste repositório para o Codeberg, nem com `git push` direto.
- Nunca use `--force`. Quando o push falha porque o Codeberg mudou, rode o comando de novo.
- Quando o comando acusa um rastro, corrija a origem no GitHub (o texto, a regra do export) e
  rode de novo. Nunca edite a cópia à mão.
- O Codeberg pode estar bloqueado na rede do agente. Nesse caso, diga ao humano que rode o
  comando na máquina dele.
