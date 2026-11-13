<!-- reading-time: 3 min -->
# Usar

## Preparar o ambiente

O Nix fixa as versões das ferramentas no `flake.lock`.

```console
nix develop
just            # lista as receitas
```

## Comandos

```console
just check                                  # formatação, build (-Werror), testes, Agda, o teste do Marola
just spec                                   # verifica a especificação em Agda
DISCORD_WEBHOOK_URL=... just test-message   # envia uma mensagem de teste ao Discord
FEED_PATH=alerts.atom just test-message     # grava o feed Atom + CAP 1.2 com um alerta de teste
just mcp                                    # servidor MCP em stdio
```

## Servidor MCP

O `just mcp` roda o servidor MCP em stdio. O servidor tem três ferramentas:

| Ferramenta | O que faz |
|---|---|
| `v0_ufs` | Lista os estados que a versão 0 atende |
| `search_sources` | Busca no catálogo de fontes oficiais, no Brasil e no mundo |
| `check_alert` | Diz se o harness da versão 0 mantém (`kept`) ou descarta (`dropped`) um alerta em JSON |

## Ferramentas do ptbr-writer

| Receita | O que faz |
|---|---|
| `just reading-time` | Grava o tempo de leitura de cada arquivo para humanos |
| `just figure` | Desenha as figuras de `docs/archify/` com o Archify |
| `just graph` | Gera o grafo do código em `build/graph/graph.html` |
| `just site` | Gera o site estático em `site/serve/alerts/` |

As ideias para hospedar o site em `piabrota.art/alerts` estão em `docs/HOSPEDAGEM.md`, no
repositório.
