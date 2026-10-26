<!-- reading-time: 2 min -->
# Contribuir

## Antes de um PR

- O repositório não tem CI. O `just check` é a verificação, e a pessoa autora roda o `just check`
  antes de cada PR.
- Uma dependência nova deve passar no `just deny`. O arquivo `deny.toml` lista as licenças aceitas.
- Um arquivo novo deve manter o `just reuse` verde.

## Pull requests

- O título do PR é em pt-BR.
- O corpo do PR é bilíngue: o pt-BR vem primeiro, e o inglês vem depois.
- Um pedido gera um PR só. Uma mudança nova para um PR aberto entra nesse PR.
- Um PR que muda uma imagem mostra uma imagem de antes e uma de depois. Essas imagens não entram
  no diff do PR.

## Regras do código

- O código não usa `unwrap` nem `expect` fora dos testes. O `unsafe` é proibido.
- Nenhum segredo entra no repositório. O webhook do Discord vem de `DISCORD_WEBHOOK_URL`.
- Toda requisição HTTP envia `smart_alerts_core::USER_AGENT`. O código nunca envia o User-Agent
  de um navegador. Quando uma fonte recusa esse nome, a pessoa autora pede acesso ao órgão.
- Um recurso pago precisa da confirmação explícita de uma pessoa.
- O símbolo ⚠ marca uma afirmação que ninguém conferiu. Ninguém remove um ⚠ sem fazer a
  conferência.

## Documentação

A documentação existe em quatro idiomas:

| Idioma | Padrão | Quem escreve |
|---|---|---|
| pt-BR | regras do ptbr-writer | a pessoa autora |
| inglês | modelo do ASD-STE100 | a pessoa autora |
| chinês | chinês simplificado controlado | tradução do inglês |
| japonês | japonês fácil (Yasashii Nihongo) | tradução do inglês |

- O `README.md` (inglês) e o `README.pt-BR.md` têm as mesmas seções e os mesmos fatos. Uma mudança
  num README muda o outro no mesmo PR.
- Uma página da wiki fica em `wiki/<idioma>/<Página>-<idioma>.md`. O sufixo de idioma deixa o nome
  da página único.
- Uma página em chinês ou em japonês começa com uma linha `<!-- source: ... sha: ... -->`. O
  `just docs-stale` lista as páginas cuja fonte em inglês mudou.
- Todo arquivo para humanos começa com `<!-- reading-time: <N> min -->`.
- Uma figura é um bloco `mermaid` seguido de um cartão *Como ler esta figura*.

## A wiki do GitHub

As páginas ficam em `wiki/`. O `just wiki-sync` copia `wiki/` para a wiki do GitHub. A receita só
roda depois que o repositório for público (issue #1).
