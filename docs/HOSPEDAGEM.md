<!-- reading-time: 5 min -->
# Hospedagem: ideias para piabrota.art/alerts

Esta página reúne ideias para publicar o ialerts no domínio `piabrota.art`. Nenhuma ideia desta
página foi adotada. O autor comprou o domínio em 2026-10-26.

## O que o autor pediu

- O endereço `piabrota.art/index.html` (a home, em HTTP ou HTTPS) sempre aponta para o site
  estático feito com Gleam.
- O ialerts fica em `piabrota.art/alerts`.
- O site usa pouco código Gleam.

## O que o repositório já tem

- O `just site` gera `site/serve/alerts/`: a página `index.html`, o estilo, os desenhos e o grafo
  do código (`grafo/graph.html`). Todo link da página é relativo, então a pasta funciona em
  `/alerts/` sem mudança.
- O código Gleam é um módulo só (`site/src/site.gleam`), sem dependência do Hex. O módulo imprime
  a página. O Nix constrói o site sem rede.
- O feed Atom com CAP 1.2 é um arquivo (`FEED_PATH`). Um servidor de arquivos estáticos pode
  servir o feed, por exemplo em `piabrota.art/alerts/feed.atom`.

## A divisão do domínio

| Caminho | Conteúdo | Origem |
|---|---|---|
| `/` e `/index.html` | O site estático em Gleam | um repositório do site (a decidir) |
| `/alerts/` | A página do ialerts e o grafo do código | `just site` deste repositório |
| `/alerts/feed.atom` | O feed Atom com CAP 1.2 | o serviço `smart-alerts`, quando existir |

Um domínio aponta para um servidor só. Então a home e o `/alerts/` saem do mesmo servidor, ou
um servidor encaminha o `/alerts/` para outro.

## Ideia 1: Codeberg Pages

- O Codeberg já guarda o espelho `piabrota/ialerts`. O Codeberg Pages serve arquivos estáticos de
  um repositório com domínio próprio ⚠ (não conferido nesta página).
- O site da home fica num repositório de páginas da conta `piabrota`. O `just site` copia
  `site/serve/alerts/` para a pasta `alerts/` desse repositório.
- O domínio próprio usa um arquivo `.domains` no repositório e um registro DNS ⚠.
- O Codeberg Pages emite o certificado HTTPS ⚠.
- **Limite:** o Codeberg Pages só serve arquivos. O serviço do Discord precisa de outro lugar
  para rodar. O feed precisa de um commit a cada atualização, ou de outro servidor.

## Ideia 2: um servidor próprio com Caddy

- Um servidor pequeno roda o binário `smart-alerts` (o serviço do Discord) e o Caddy (Apache-2.0).
- O Caddy serve `/` com o site em Gleam e `/alerts/` com `site/serve/alerts/`. O Caddy também
  serve o feed que o serviço grava.
- O Caddy pede e renova o certificado HTTPS sozinho, e redireciona HTTP para HTTPS ⚠.
- **Limite:** um servidor tem custo mensal. Um recurso pago precisa da confirmação explícita do
  autor.

Um `Caddyfile` mínimo para essa ideia ⚠ (não testado):

```
piabrota.art {
    handle_path /alerts/* {
        root * /srv/alerts
        file_server
    }
    handle {
        root * /srv/home
        file_server
    }
}
```

## Ideia 3: GitHub Pages

- O GitHub Pages serve arquivos estáticos com domínio próprio e HTTPS ⚠.
- **Limite:** o repositório é privado. O GitHub Pages de um repositório privado pode exigir um
  plano pago ⚠. A regra da Wiki (issue #1) também espera o repositório público.

## Recomendação do autor desta página

A ideia 1 começa sem custo e combina com o espelho no Codeberg. A ideia 2 entra quando o serviço
do Discord precisar de um servidor: o mesmo servidor passa a servir o site e o feed. Essa
recomendação é uma leitura, não um fato conferido.

## Próximos passos

1. Escolher onde fica o código da home (`piabrota.art/`).
2. Escolher a ideia de hospedagem.
3. Criar os registros DNS no registrador do domínio.
4. Conferir os ⚠ desta página antes de publicar.

## Afirmações ⚠ desta página

- O Codeberg Pages aceita domínio próprio com um arquivo `.domains` e emite HTTPS.
- O Caddy emite e renova o certificado e redireciona HTTP para HTTPS.
- O `Caddyfile` acima funciona sem mudança.
- O GitHub Pages serve domínio próprio com HTTPS, e um repositório privado pode exigir plano pago.
