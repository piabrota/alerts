<!-- reading-time: 1 min -->
# Usar

## Preparar o ambiente

O Nix fixa as versões das ferramentas no `flake.lock`.

```console
nix develop
just            # lista as receitas
```

## Comandos

```console
just check                                  # formatação, clippy, testes, licenças, traduções
DISCORD_WEBHOOK_URL=... just test-message   # envia uma mensagem de teste ao Discord
FEED_PATH=alerts.atom just test-message     # grava o feed Atom + CAP 1.2 com um alerta de teste
just mcp                                    # servidor MCP em stdio
just py-build                               # gera o pacote Python
```

## Pacote Python

O `just py-build` gera o pacote com o maturin. O módulo se chama `smart_alerts`.

```python
import smart_alerts

smart_alerts.__version__       # a versão do pacote
smart_alerts.v0_ufs()          # ['PR', 'SC', 'RS']
smart_alerts.parse_uf(" rs ")  # 'RS'
```

- A função `v0_ufs` devolve os estados que a versão 0 atende.
- A função `parse_uf` normaliza um código de estado. A função levanta `ValueError` para um código
  desconhecido.

## Servidor MCP

O `just mcp` roda o servidor MCP em stdio. O servidor tem duas ferramentas:

| Ferramenta | O que faz |
|---|---|
| `v0_ufs` | Lista os estados que a versão 0 atende |
| `check_alert` | Diz se o harness da versão 0 mantém (`kept`) ou descarta (`dropped`) um alerta em JSON |
