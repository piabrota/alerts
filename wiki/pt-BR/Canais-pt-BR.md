<!-- reading-time: 2 min -->
# Canais

Um canal entrega os alertas que saem do harness. A versão 0 tem dois canais.

| Canal | Padrão | Entrega |
|---|---|---|
| Discord | webhook do Discord | push |
| Feed | Atom (RFC 4287) com CAP 1.2 (OASIS) em cada entrada | pull |

## Paridade

- Os dois canais leem a mesma saída do harness.
- Um alerta que chega ao Discord também chega ao feed, com o mesmo texto em pt-BR.
- Um canal nunca filtra nem reescreve um alerta. Um canal novo também lê a mesma saída.

## Discord

- O servidor do Discord já existe.
- O endereço do webhook vem da variável de ambiente `DISCORD_WEBHOOK_URL`. O endereço nunca entra
  no repositório.

## Feed Atom com CAP 1.2

- Atom e CAP são padrões abertos, sem empresa por trás.
- O feed tem o mesmo formato do feed nacional da Defesa Civil (IDAP). Qualquer leitor de feed abre
  o arquivo.
- A saída CAP deve passar na validação do XSD do CAP 1.2 da OASIS.
- Um alerta de teste sai sempre com `<status>Test</status>`.
- O caminho do arquivo vem da variável de ambiente `FEED_PATH`.

## Texto dos alertas

Na versão 0, o texto dos alertas é só em pt-BR, no nível `estrito` do ptbr-writer:

- O texto começa pelo risco e pelo lugar: `Risco de inundação em Porto Alegre (RS).`
- O texto usa as palavras de severidade da fonte, sem aumentar nem diminuir a severidade.
- Uma ação aparece só quando a fonte dá essa ação.

## Consumidores planejados

| Consumidor | Como lê o alerts |
|---|---|
| [AstrBot](https://github.com/AstrBotDevs/AstrBot) | o pacote Python `smart-alerts` num plugin |
| [astrbot_plugin_disaster_warning](https://github.com/Pancakes-Labs/astrbot_plugin_disaster_warning) | o pacote Python ou o feed, para alertas do Brasil |

Um plugin chama o pacote. O alerts não copia código AGPL-3.0.
