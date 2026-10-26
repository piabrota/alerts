# ialerts

🇬🇧 [Read in English](README.md) · nome de código: `smart-alerts`

Alertas do século 22.

## em um minuto

- **O ialerts é uma plataforma de alertas genérica.** Ele coleta alertas de fontes oficiais,
  filtra os alertas e entrega os alertas. Um risco, uma região ou um canal é um módulo: o núcleo
  não muda quando alguém adiciona um módulo.
- **Só fontes oficiais.** Um alerta vem de um órgão oficial. O ialerts nunca inventa um alerta,
  uma área, um horário ou uma ação que a fonte não diz.
- **O primeiro risco é inundação no Brasil.** A v0 atende a Região Sul: Paraná (PR),
  Santa Catarina (SC) e Rio Grande do Sul (RS).
- **A v0 entrega um serviço para Discord** e um feed Atom com alertas CAP 1.2, com os mesmos
  alertas e o mesmo texto. O pacote Python `smart-alerts` existe desde o
  início. Um plugin AstrBot vai usar esse pacote no futuro.
- **Aberto por princípio.** Padrões abertos (Atom, CAP 1.2), só dependências FOSS, só modelos
  locais. Uma API paga nunca é obrigatória.
- **Estado:** bootstrap. Nenhuma fonte está implementada. O serviço só envia uma mensagem de teste.

O código, os crates e o binário ainda usam o nome de código `smart-alerts`.

## fontes

Os dados do Brasil estão espalhados. Cada fonte é um módulo próprio: adicionar uma fonte não
muda o núcleo.

Verificação de 2026-10-22. Cada endpoint foi acessado duas vezes: de dentro do Brasil (`curl`) e
de fora do Brasil (um cliente HTTP em servidor no exterior). Nenhuma fonte bloqueia acesso por
país. Nenhuma fonte publica regra sobre acesso estrangeiro. O endpoint, o formato e a autenticação de
cada fonte estão em [`docs/SOURCES.md`](docs/SOURCES.md).

| Fonte | API oficial | Formato | Autenticação | Fora do Brasil | Versão |
|---|---|---|---|---|---|
| Defesa Civil (IDAP) | sim, sem documentação ⚠ | Atom com CAP 1.2 | nenhuma | acessível | v0 |
| INMET | sim | RSS e CAP 1.2; JSON sem documentação | nenhuma (avisos); token por e-mail (estações) | acessível | v0 |
| CEMADEN | sim (observações) | REST JSON | cadastro gratuito (JWT) | acessível | v0 |
| ANA | sim | REST JSON; SOAP legado | cadastro com CPF/CNPJ (REST); nenhuma (SOAP) | acessível; o cadastro pode exigir CPF/CNPJ ⚠ | planejada |
| CPTEC/INPE | sim, só previsão | XML | nenhuma | acessível; uso não comercial | planejada |
| Epagri/Ciram (SC) | parcial | RSS e JSON do WordPress, texto livre | nenhuma (avisos); CPF (dados de estação) | acessível | planejada |
| SGB (SACE) | não | HTML; boletins em PDF | nenhuma | acessível | planejada |
| Defesa Civil RS | não | PDF; JSON do ClimaRS sem documentação | nenhuma | acessível | planejada |
| SIMEPAR (PR) | não encontrada | — | — | bloqueado em qualquer país (Cloudflare) | planejada |

**O que cada fonte exige do código:**

- **Defesa Civil (IDAP):** o feed `idapfile.mdr.gov.br/idap/api/rss/cap` guarda cerca de 12 horas
  de alertas ⚠ e tem cerca de 10 MB. O módulo deve consultar o feed com frequência e guardar os
  alertas. Os alertas de PR, SC e RS aparecem nesse feed. A Defesa Civil de SC também publica um
  RSS em texto livre.
- **INMET:** `apiprevmet3.inmet.gov.br` derruba a conexão quando o User-Agent é o padrão do curl
  (`curl/...`). O smart-alerts se identifica com o próprio nome (`smart-alerts/<versão>`), e o
  INMET aceita esse nome. O RSS declara domínio público, com citação da fonte.
- **CEMADEN:** a API PED (`sws.cemaden.gov.br/PED`) entrega observações, não alertas. Os alertas
  municipais só existem num JSON sem documentação (`painelalertas.cemaden.gov.br/wsAlertas2`). O
  limite é de 12 requisições por minuto para usuário externo ⚠. O CEMADEN exige citação.
- **ANA:** o token do REST vale 60 minutos. Pedidos de token frequentes podem bloquear o IP.
- **CPTEC/INPE:** os termos proíbem uso comercial sem autorização e exigem a citação
  "CPTEC/INPE". O serviço não tem avisos.
- **Defesa Civil RS:** substituiu a Sala de Situação da SEMA-RS em fevereiro de 2025. O JSON do
  ClimaRS (níveis de rio da ANA) pode mudar sem aviso.
- **SIMEPAR:** nenhum cliente automático passa pelo desafio da Cloudflare. A fonte fica sem módulo
  até o SIMEPAR publicar uma API.

## modelos locais

Algumas fontes publicam texto livre (boletins, PDFs). Para essas fontes, o smart-alerts vai usar
modelos abertos que rodam na máquina local. Uma API paga nunca é obrigatória.

| Runtime | Licença | Uso no smart-alerts | Estado |
|---|---|---|---|
| [Ollama](https://github.com/ollama/ollama) | MIT | Ler texto livre e extrair campos (local, hora, nível do rio) para um `Alert`. | planejado |
| [OLLAYA](https://github.com/ollaya-dev/ollaya) | Apache-2.0 | Decidir: perguntas tipadas (é alerta de inundação? qual severidade?) com probabilidade calibrada. Não gera texto. | em breve |

Um modelo nunca inventa alerta. Quando o modelo não tem certeza, o alerta volta para o harness
determinístico e fica marcado para revisão humana.

## canais

| Canal | Padrão | Entrega |
|---|---|---|
| Discord | webhook do Discord | push |
| Feed | Atom (RFC 4287) com CAP 1.2 (OASIS) em cada entrada | pull |

Os dois canais leem a mesma saída do harness: um alerta que chega ao Discord também chega ao
feed, com o mesmo texto. O feed não depende de empresa: Atom e CAP são padrões abertos. O feed
tem o mesmo formato do feed nacional da Defesa Civil (IDAP), e qualquer leitor de feed abre o
arquivo. Um alerta de teste sai com `<status>Test</status>`.

## quem vai usar o ialerts

| Consumidor | O que é | Como lê o ialerts | Situação |
|---|---|---|---|
| Discord | O servidor da v0, que já existe | webhook (push) | v0 |
| Qualquer leitor de feed | Atom (RFC 4287) com CAP 1.2 em cada entrada | o arquivo do feed (pull) | v0 |
| [AstrBot](https://github.com/AstrBotDevs/AstrBot) | Plataforma livre de chatbot para apps de mensagem (AGPL-3.0) | o pacote Python `smart-alerts` num plugin | planejado |
| [astrbot_plugin_disaster_warning](https://github.com/Pancakes-Labs/astrbot_plugin_disaster_warning) | Plugin de alertas de desastre para o AstrBot: terremoto, tsunami, tempo (AGPL-3.0) | o pacote Python ou o feed, para alertas do Brasil | planejado |

Um plugin chama o pacote: o ialerts não copia código AGPL. Mais em
[`docs/REFERENCES.md`](docs/REFERENCES.md).

## arquitetura

```mermaid
%% figure: crates
%% title: Quais crates existem e quem depende de quem?
flowchart LR
    S["smart-alerts-source-*<br/>(uma por fonte)"] --> C["smart-alerts-core<br/>Alert, Uf, Harness"]
    D["smart-alerts<br/>serviço"] --> C
    D --> F["smart-alerts-feed<br/>Atom + CAP 1.2"] --> C
    P["smart-alerts-py<br/>pacote Python"] --> C
    M["smart-alerts-mcp<br/>servidor MCP"] --> C
```

<details open>
<summary>Como ler esta figura</summary>

**Em uma frase.** Todo crate depende do `smart-alerts-core`. O núcleo não depende de nenhum crate.

**Como ler.** Cada seta vai do crate que usa para o crate usado.

**Fora da figura.** As crates de fonte ainda não existem. O harness compõe estágios puros
(`Fn(Alert) -> Option<Alert>`): veja `GROUND_TRUTH.md`.

**Evidência.** `Cargo.toml` de cada crate em `crates/`.

</details>

## usar

```console
nix develop
just check                                  # formatação, clippy, testes, licenças
DISCORD_WEBHOOK_URL=... just test-message   # envia uma mensagem de teste ao Discord
FEED_PATH=alerts.atom just test-message     # grava o feed Atom + CAP 1.2 com um alerta de teste
just mcp                                    # servidor MCP em stdio
just py-build                               # gera o pacote Python
```

## documentação

A documentação existe em quatro idiomas: pt-BR, inglês, chinês e japonês. As páginas estão em
`wiki/`. A Wiki do GitHub só recebe as páginas depois que o repositório for público (issue #1).

## créditos

- O layout do README em inglês vem do [picasso](https://github.com/h0ffmann/picasso).
- As convenções de app (geradores, doctor, agendador, tarefas) seguem o [Loco](https://loco.rs)
  (Apache-2.0). As capturas de tela do bootstrap estão em [`docs/screenshots/loco-bootstrap/`](docs/screenshots/loco-bootstrap).
- Os mapas de alerta vão usar o [MapLibre](https://maplibre.org) (BSD-3-Clause) e dados do OpenStreetMap (ODbL).

- O harness funcional traduz o
  [Object-Oriented Harness](https://arxiv.org/abs/2610.28380) (arXiv 2610.28380) para composição
  de funções.
- A escrita em pt-BR segue o [ptbr-writer](https://github.com/h0ffmann/ptbr-writer). O verbo `open-pr` é a skill
  [`bipr`](https://github.com/h0ffmann/ptbr-writer/blob/main/skills/bipr/SKILL.md) dele, adaptada (MPL-2.0).
- A escrita em inglês segue o modelo do ASD-STE100.
- O japonês segue o japonês fácil (Yasashii Nihongo).

## como citar

Use o [`CITATION.cff`](CITATION.cff), com o commit ou a tag usada. O [`REUSE.toml`](REUSE.toml)
dá a licença e o copyright de cada arquivo, e as regras de citação para agentes.

## licença

[MPL-2.0](LICENSE).
