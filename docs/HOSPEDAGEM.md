<!-- reading-time: 5 min -->
# Hospedagem: piabrota.art no Codeberg Pages

Esta página diz onde o site estático do ialerts fica hospedado e como publicar o site. O autor
comprou o domínio `piabrota.art` em 2026-10-26.

## O que o autor pediu

- O endereço `piabrota.art/index.html` (a home, em HTTP ou HTTPS) sempre aponta para o site
  estático feito com Gleam.
- O ialerts fica em `piabrota.art/alerts/`.
- O site usa pouco código Gleam.
- A hospedagem é grátis.

## O que o `just site` gera

| Caminho | Conteúdo | Código |
|---|---|---|
| `/index.html` | A home de piabrota.art | `site/src/home.gleam` |
| `/alerts/` | A página do ialerts, a figura e o desenho | `site/src/site.gleam` |
| `/alerts/grafo/graph.html` | O grafo do código | `code-graph` |

O código Gleam são dois módulos pequenos, sem dependência do Hex. Todo link é relativo.

## Comparação das hospedagens grátis

Conferido em 2026-10-26, nas páginas citadas.

| Hospedagem | Grátis para este caso | Deploy sem CI | Domínio próprio e HTTPS | Observação |
|---|---|---|---|---|
| Codeberg Pages | sim, para um repositório público ([docs](https://docs.codeberg.org/codeberg-pages/)) | sim: um push no branch `pages` e um webhook | sim, com DNS e Let's Encrypt ([docs](https://docs.codeberg.org/codeberg-pages/using-custom-domain/)) | Codeberg e.V., associação sem fins lucrativos; o espelho `piabrota/alerts` já fica no Codeberg |
| GitHub Pages | não: o plano Free só publica Pages de repositório público, e `h0ffmann/smart-alerts` é privado ([docs](https://docs.github.com/en/get-started/learning-about-github/githubs-plans)) | só com outro repositório público ou com Actions | sim | o deploy comum usa Actions, e o repositório proíbe CI |
| Cloudflare Pages | sim, no plano grátis ⚠ | sim, com o CLI `wrangler` ⚠ | sim ⚠ | empresa; pede conta e o CLI do npm |
| Netlify | sim, no plano grátis ⚠ | sim, com o CLI `netlify` ⚠ | sim ⚠ | empresa; pede conta e o CLI do npm |

**Decisão: Codeberg Pages, só com o domínio piabrota.art.** É grátis, não depende de empresa, não
pede CI e combina com o espelho no Codeberg. O endereço do site é só `piabrota.art`.

## Como publicar

O deploy roda na máquina do autor, porque o repositório não tem CI.

### Uma vez

1. No Codeberg, crie o repositório público `piabrota/pages` (vazio).
2. Em **Settings → Webhooks → Add webhook → Forgejo**, crie um webhook:
   - **Target URL:** `http://piabrota.art/` na primeira vez. Depois do primeiro deploy, troque
     para `https://piabrota.art/`: o Codeberg só emite o certificado de um domínio que já recebeu
     um deploy.
   - **Branch filter:** `pages`.
   - Não use o botão "Test delivery": a documentação diz que o teste falha.
3. Crie os registros DNS abaixo no registrador de `piabrota.art`.

O site só abre em `piabrota.art`. O endereço `piabrota.codeberg.page` só existe com um webhook
próprio, e esta configuração não cria esse webhook: nenhum domínio de empresa entra no endereço.

### A cada deploy

```console
just deploy-site
```

A receita roda o `just site`, confere que o site não tem rastro de IA (`export-codeberg --check`) e
faz um push forçado de um commit só para o branch `pages` de `piabrota/pages`. O branch guarda só
o site gerado, nunca o histórico deste repositório. O commit leva a identidade git da máquina.

## Registros DNS de piabrota.art

Fonte: [Using custom domains](https://docs.codeberg.org/codeberg-pages/using-custom-domain/),
lida em 2026-10-26.

| Nome | Tipo | Valor | Quando |
|---|---|---|---|
| `piabrota.art` | `ALIAS` (ou CNAME "achatado") | `codeberg.page` | o registrador aceita ALIAS |
| `piabrota.art` | `A` | `217.197.84.141` | sem ALIAS, ou com DNSSEC |
| `piabrota.art` | `AAAA` | `2a0a:4580:103f:c0de::2` | sem ALIAS, ou com DNSSEC |
| `_git-pages-repository.piabrota.art` | `TXT` | `https://codeberg.org/piabrota/pages.git` | sempre |
| `www.piabrota.art` | `CNAME` | `codeberg.page` | se quiser o `www` |
| `_git-pages-repository.www.piabrota.art` | `TXT` | `https://codeberg.org/piabrota/pages.git` | se quiser o `www` |

- Use o ALIAS ou o par A e AAAA, não os dois. A documentação avisa que os IPs podem mudar, então
  o ALIAS é melhor quando o registrador aceita.
- A palavra `repository` no nome do TXT é literal.
- Um registro CAA, se existir, precisa liberar o Let's Encrypt. Os valores estão na documentação.
- O `www` precisa de um webhook próprio, com Target URL `http://www.piabrota.art/`.

## Depois: o serviço do Discord

O Codeberg Pages só serve arquivos. O serviço `smart-alerts` (o Discord e o feed que muda a cada
alerta) precisa de um servidor quando existir. Uma ideia é um servidor pequeno com o Caddy
(Apache-2.0): o Caddy serve o feed e pode passar a servir o site. Um servidor tem custo mensal, e
um recurso pago precisa da confirmação explícita do autor.

## Afirmações ⚠ desta página

- Os planos grátis, o deploy por CLI e o HTTPS do Cloudflare Pages e do Netlify (não conferidos
  nesta data).
