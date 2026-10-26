<!-- reading-time: 2 min -->
# Fontes

Os dados do Brasil estão espalhados. No alerts, cada fonte é um crate próprio,
`crates/alerts-source-<id>`. Esse crate transforma os dados do órgão em alertas. Uma fonte
nova ou um estado novo adiciona um crate e não muda o núcleo.

A conferência é de 2026-10-22. Cada endpoint foi acessado de dentro do Brasil (`curl`) e de fora
do Brasil (um cliente HTTP num servidor no exterior). Nenhuma fonte bloqueia acesso por país. O
endpoint, o formato e a autenticação de cada fonte estão em `docs/SOURCES.md`, no repositório.

| Fonte | API oficial | Formato | Autenticação | Versão |
|---|---|---|---|---|
| Defesa Civil (IDAP) | sim, sem documentação ⚠ | Atom com CAP 1.2 | nenhuma | v0 |
| INMET | sim | RSS e CAP 1.2; JSON sem documentação | nenhuma (avisos); token por e-mail (estações) | v0 |
| CEMADEN | sim (observações) | REST JSON | cadastro gratuito (JWT) | v0 |
| ANA | sim | REST JSON; SOAP legado | cadastro com CPF ou CNPJ (REST); nenhuma (SOAP) | planejada |
| CPTEC/INPE | sim, só previsão | XML | nenhuma | planejada |
| Epagri/Ciram (SC) | parcial | RSS e JSON do WordPress, texto livre | nenhuma (avisos); CPF (dados de estação) | planejada |
| SGB (SACE) | não | HTML; boletins em PDF | nenhuma | planejada |
| Defesa Civil RS | não | PDF; JSON do ClimaRS sem documentação | nenhuma | planejada |
| SIMEPAR (PR) | não encontrada | — | — | planejada |

## O que cada fonte exige do código

- **Defesa Civil (IDAP).** O feed guarda cerca de 12 horas de alertas ⚠ e tem cerca de 10 MB. O
  módulo deve consultar o feed com frequência e guardar os alertas que leu.
- **INMET.** O servidor derruba a conexão quando o User-Agent é o padrão do curl. O servidor
  aceitou o User-Agent honesto com o nome antigo do projeto em 2026-10-22. O
  nome novo é `piabrota-alerts/<versão>`, e ninguém conferiu esse nome ⚠. O RSS declara domínio público, com citação da fonte.
- **CEMADEN.** A API oficial entrega observações, não alertas. Os alertas municipais só existem
  num JSON sem documentação (`wsAlertas2`). O limite é de 12 requisições por minuto para um
  usuário externo ⚠. O CEMADEN exige citação.
- **ANA.** O token do REST vale 60 minutos. Pedidos de token frequentes podem bloquear o IP. Um
  cadastro sem CPF ou CNPJ pode não ser aceito ⚠.
- **CPTEC/INPE.** Os termos proíbem uso comercial sem autorização e exigem a citação
  "CPTEC/INPE". O serviço só tem previsão, sem avisos.
- **Defesa Civil RS.** A Defesa Civil RS substituiu a Sala de Situação da SEMA-RS em fevereiro de
  2025. O JSON do ClimaRS pode mudar sem aviso.
- **SIMEPAR.** Nenhum cliente automático passa pelo desafio da Cloudflare. A fonte fica sem módulo
  até o SIMEPAR publicar uma API.

## Endpoints sem documentação

Um endpoint sem documentação (CEMADEN `wsAlertas2`, INMET `avisos/ativos`, ClimaRS) pode mudar sem
aviso. Um módulo que usa um desses endpoints deve falhar de forma visível, nunca em silêncio.
