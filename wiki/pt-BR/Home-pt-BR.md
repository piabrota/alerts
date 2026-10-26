<!-- reading-time: 2 min -->
# alerts

O alerts é uma plataforma de alertas genérica.

O alerts coleta alertas de fontes oficiais, filtra os alertas e entrega os alertas. Um risco, uma
região ou um canal é um módulo. O núcleo não muda quando alguém adiciona um módulo.

## A versão 0

- **Risco:** inundação no Brasil.
- **Região:** a Região Sul, com três estados: Paraná (PR), Santa Catarina (SC) e Rio Grande do Sul
  (RS).
- **Fontes:** Defesa Civil, INMET e CEMADEN.
- **Canais:** um serviço para Discord e um feed Atom com alertas CAP 1.2. Os dois canais recebem os
  mesmos alertas, com o mesmo texto em pt-BR.
- **Pacote Python:** o pacote `alerts` existe desde o início. Um plugin do AstrBot vai usar
  esse pacote no futuro.

## Estado

O projeto está na fase de bootstrap. Nenhuma fonte está implementada. O serviço só envia uma
mensagem de teste.

## Princípios

- **Só fontes oficiais.** Um alerta vem de um órgão oficial. O alerts nunca inventa um alerta,
  uma área, um horário ou uma ação que a fonte não diz.
- **Identificação honesta.** Toda requisição HTTP envia o User-Agent `piabrota-alerts/<versão>`. O
  alerts nunca se passa por um navegador.
- **Aberto por princípio.** O alerts usa padrões abertos (Atom, CAP 1.2), só dependências FOSS e
  só modelos locais. Uma API paga nunca é obrigatória.

## Páginas

| Página | Assunto |
|---|---|
| [Fontes](Fontes-pt-BR) | As fontes oficiais, o formato e a autenticação de cada uma |
| [Canais](Canais-pt-BR) | O Discord e o feed Atom com CAP 1.2 |
| [Arquitetura](Arquitetura-pt-BR) | Os crates e o harness funcional |
| [Usar](Usar-pt-BR) | Os comandos, o pacote Python e o servidor MCP |
| [Contribuir](Contribuir-pt-BR) | As regras para uma mudança no repositório |
| [Citar](Citar-pt-BR) | A citação, a licença e os créditos |
