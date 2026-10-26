<!-- reading-time: 5 min -->
# Referências: ferramentas e serviços externos

Esta página lista ferramentas e serviços externos que alguém propôs para o smart-alerts. Esta
página não lista fontes de alerta: as fontes estão em `docs/SOURCES.md`.

Uma entrada desta página não foi adotada. Cada entrada diz o que o recurso é, a licença, o custo e
a relevância. Uma dependência nova ainda precisa passar no `just deny`. Um recurso pago precisa da
confirmação explícita de um humano (`GROUND_TRUTH.md`).

Conferido em 2026-10-26 (AstrBot e plugin de desastres) e em 2026-10-22 (Onyx e Seabase).

| Recurso | O que é | Licença e custo | Relevância | Situação |
|---|---|---|---|---|
| [AstrBot](https://github.com/AstrBotDevs/AstrBot) | Plataforma livre de chatbot com agente, para apps de mensagem | AGPL-3.0, gratuito | Vai receber os alertas do smart-alerts por um plugin | consumidor planejado |
| [astrbot_plugin_disaster_warning](https://github.com/Pancakes-Labs/astrbot_plugin_disaster_warning) | Plugin de alertas de desastre para o AstrBot | AGPL-3.0, gratuito | É o exemplo de consumidor citado no `GROUND_TRUTH.md` | consumidor planejado |
| [Onyx](https://docs.onyx.app/deployment/getting_started/quickstart) | Plataforma de chat e busca com IA, auto-hospedada ([`onyx-dot-app/onyx`](https://github.com/onyx-dot-app/onyx)) | MIT, exceto os diretórios `ee` (Onyx Enterprise License) | Pode indexar a documentação e as fontes deste repositório para busca | não adotado |
| [Seabase](https://seabase.ai/infrastructure) | Data centers submarinos para IA | comercial ⚠ (a página não mostra preço) | Só hospedagem; a v0 não precisa | não adotado |

## AstrBot

- **O que é.** O AstrBot é uma plataforma de chatbot de código aberto. A plataforma liga um LLM
  (com agente, MCP e base de conhecimento) aos apps de mensagem. O README cita QQ, WeChat Work,
  Feishu, DingTalk, Telegram, Slack e Discord (suporte oficial).
- **Plugins.** O AstrBot tem um mercado com mais de 1000 plugins, instalados com um clique. Um
  plugin é escrito em Python. O AstrBot pede Python 3.12 ou mais novo.
- **Licença.** O arquivo `LICENSE` do repositório é a AGPL-3.0.
- **Relação com o smart-alerts.** O pacote `smart-alerts-py` existe para um plugin do AstrBot
  usar. O plugin chama o pacote: o smart-alerts não copia código do AstrBot. A licença AGPL-3.0
  não é compatível com uma cópia de código dentro de um repositório MPL-2.0.
- **Não conferido:** como um plugin publica uma mensagem no Discord pelo AstrBot ⚠.

## astrbot_plugin_disaster_warning

- **O que é.** O plugin manda alertas de terremoto, tsunami, tempo e tufão para os chats do
  AstrBot, em tempo real. O plugin tem uma WebUI e mostra mapas.
- **Repositório.** O repositório ativo é o `Pancakes-Labs/astrbot_plugin_disaster_warning`. Os
  autores são DBJD-CR e Aloys233. A versão em 2026-10-26 é a v1.6.5, e o plugin pede o AstrBot
  4.11.2 ou mais novo. Existem cópias (forks) em outras contas, por exemplo
  `Aloys233/astrbot_plugin_disaster_warning` e `vtloader/astrbot_plugin_disaster_warning`.
- **Fontes do plugin.** O plugin usa a rede sismológica da China (CENC), o serviço meteorológico
  da China (CMA), o serviço meteorológico de Taiwan (CWA), a agência meteorológica do Japão (JMA),
  o USGS e o Global Quake. Nenhuma fonte do plugin cobre o Brasil ⚠ (a conferência foi por busca
  de texto no README).
- **Licença.** O arquivo `LICENSE` é a AGPL-3.0. A descrição de alguns forks diz que o plugin foi
  feito com IA.
- **Relação com o smart-alerts.** O plugin pode receber alertas do Sul do Brasil pelo
  `smart-alerts-py` ou pelo feed Atom com CAP 1.2. O texto do alerta continua em pt-BR, igual ao
  texto do Discord. O smart-alerts não reescreve um alerta para um canal.
- **Não conferido:** se o plugin aceita uma fonte nova sem mudança no código do plugin ⚠.

## Onyx

- **Licença.** O arquivo `LICENSE` da raiz coloca todo o conteúdo fora dos diretórios `ee` sob a
  licença MIT (Expat). Os diretórios `ee` (`backend/ee`, `web/src/app/ee`, `web/src/ee`) estão sob a
  Onyx Enterprise License. Uma instalação para o smart-alerts usa só a parte MIT ⚠ (não conferido:
  quais funções estão no `ee`).
- **Instalação** (da página de início rápido):

  ```bash
  uv tool install onyx-cli && onyx-cli deploy install
  ```

  O instalador confere os recursos do sistema e pergunta o modo (Lite ou Standard) e a versão. O
  instalador grava a configuração em `~/.config/onyx` e inicia os contêineres. No Linux, o
  instalador instala o Docker quando o Docker falta.
- **Fora da página de início rápido:** a memória e a CPU necessárias, os modelos suportados (um
  modelo local pelo Ollama ⚠), os conectores e o funcionamento da indexação.
- **Regras do repositório.** O Onyx é um serviço separado, não uma dependência Rust, então o
  `just deny` não confere o Onyx. Rode o Onyx na máquina local. Não coloque o Onyx no repositório
  como código ou como CI.
- **Antes de usar:** leia o `docs/SURVEY_NLP.md`. Use a ferramenta mais barata que resolve o
  problema.

## Seabase

- A página é uma aplicação JavaScript. O HTML da página só tem um título e uma descrição:
  "Modular coastal and marine AI infrastructure designed for high-density GPU clusters, regional
  placement, and incremental expansion near demand."
- A página não mostra preço, API, licença nem opção de auto-hospedagem ⚠.
- A v0 do smart-alerts é um binário estático e não precisa de GPU. Um serviço de hospedagem pago
  precisa da confirmação explícita de um humano.
- Uso possível: o marola (trabalho no oceano), não o smart-alerts ⚠. Ninguém decidiu isso.

## Afirmações ⚠ desta página

- Como um plugin do AstrBot publica uma mensagem no Discord.
- Nenhuma fonte do plugin de desastres cobre o Brasil.
- O plugin de desastres aceita uma fonte nova sem mudança no código.
- Quais funções do Onyx estão nos diretórios `ee`.
- Suporte do Onyx a um modelo local pelo Ollama.
- Preço, API e licença do Seabase.
- O Seabase como opção para o marola.
