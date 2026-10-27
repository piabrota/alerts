//! The about page: what this app is inside marola v2, the promises it keeps, the repos and the
//! credits. The promises are the hard rules of `GROUND_TRUTH.md` and the marola philosophy
//! (`marola-dev/marola` `PHILOSOPHY.md`, marola.dev `about.html`), in the app's words.

use crate::marola::{ALERTS, AREAS, LOGO, Out, Pill, REPOS};
use leptos::prelude::*;

/// A promise: a short title and one sentence.
const PROMISES: &[(&str, &str)] = &[
    (
        "Nunca inventa.",
        "Todo alerta vem de uma fonte oficial (Defesa Civil, INMET, CEMADEN). O app não cria alerta, área, horário nem ação que a fonte não diz.",
    ),
    (
        "Diz quando não sabe.",
        "Uma UF sem alerta aparece como \"sem alerta\", um registro que o mapa não lê é contado na tela, e um erro de leitura aparece como aviso, nunca como uma lista vazia.",
    ),
    (
        "Regras que qualquer pessoa lê.",
        "O filtro e a rota de cada alerta são código simples. Um modelo de decisão só vai responder perguntas tipadas, com probabilidade, e nunca escreve o texto do alerta.",
    ),
    (
        "Identifica-se com honestidade.",
        "Toda requisição leva o nome piabrota-alerts e o endereço do repositório. Quando uma fonte recusa esse nome, a gente pede o acesso à agência, sem disfarce.",
    ),
    (
        "Aberto, sem conta e sem chave.",
        "O código é MPL-2.0, o mapa é software livre, e nenhuma API paga entra sem uma pessoa aprovar. Os alertas ficam num banco no seu computador.",
    ),
];

#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <article class="about-page">
            <header class="about-hero">
                <img src=LOGO alt="O logo do marola: uma onda quebrando dentro de um disco, com um check no tubo" width="112" height="112" />
                <div>
                    <p class="eyebrow">"marola v2 · alertas"</p>
                    <h1>"Sobre os alertas do marola"</h1>
                    <p class="lede">
                        "O marola é um guia amigável para a água perto de você, feito em código aberto, como ciência cidadã. Este app é a seção de alertas: no marola.dev ela ainda aparece como \"em breve\"."
                    </p>
                </div>
            </header>

            <section>
                <h2>"O que ele faz"</h2>
                <p>
                    "Ele junta os alertas oficiais de inundação, filtra e entrega os mesmos alertas, com o mesmo texto, no Discord, num feed Atom com CAP 1.2 e neste mapa. O primeiro recorte é o Sul: Paraná, Santa Catarina e Rio Grande do Sul."
                </p>
            </section>

            <section>
                <h2>"Promessas"</h2>
                <ul class="promises">
                    {PROMISES.iter().map(|&(title, text)| view! {
                        <li><b>{title}</b>" "{text}</li>
                    }).collect_view()}
                </ul>
            </section>

            <section>
                <h2>"Por que aberto e sem fins lucrativos"</h2>
                <p>
                    "Boa parte dos dados sobre a água no Brasil está com quem é dono deles, e sem esses dados ninguém prevê desastre climático. Uma organização sem fins lucrativos que promete devolver tudo pode pedir esses dados. Com o código aberto, o projeto segue vivo mesmo quando quem começou vai embora. "
                    <Out href="https://marola.dev/about.html".to_owned()>"Mais em marola.dev/sobre"</Out>
                    "."
                </p>
            </section>

            <section>
                <h2>"As áreas do marola"</h2>
                <p>
                    "O mapa do mar do marola.dev cobre três áreas, e o seletor de área do mapa de alertas leva a elas: "
                    {AREAS.iter().map(|a| a.name).collect::<Vec<_>>().join(", ")}
                    ". Um alerta continua marcado por UF: a área só move o mapa."
                </p>
            </section>

            <section>
                <h2>"Os repositórios"</h2>
                <ul class="repos">
                    <li><Pill repo=ALERTS /><span>{ALERTS.about}</span></li>
                    {REPOS.iter().map(|&r| view! {
                        <li><Pill repo=r /><span>{r.about}</span></li>
                    }).collect_view()}
                </ul>
            </section>

            <section>
                <h2>"Créditos"</h2>
                <ul class="credits">
                    <li>"Logo: marola v2, de Matheus Hoffmann, desenhado com equações no Desmos (CC BY 4.0)."</li>
                    <li>
                        <Out href="https://maplibre.org/".to_owned()>"MapLibre GL JS"</Out>
                        " desenha o mapa, sobre o estilo positron do "
                        <Out href="https://openfreemap.org/".to_owned()>"OpenFreeMap"</Out>
                        ". Dados do mapa © "
                        <Out href="https://www.openstreetmap.org/copyright".to_owned()>"colaboradores do OpenStreetMap"</Out>
                        " (ODbL). O OpenFreeMap recebe o pedido de cada pedaço do mapa; nenhum outro serviço é chamado."
                    </li>
                    <li>"Limites das UFs: IBGE, API de malhas."</li>
                    <li>
                        <Out href="https://github.com/ollaya-dev/ollaya".to_owned()>"Ollaya"</Out>
                        " (projeto FOSS, Apache-2.0): o modelo de decisão planejado, para perguntas como \"é alerta de inundação?\"."
                    </li>
                    <li>"Leptos e Tauri fazem a interface e a janela."</li>
                </ul>
                <p class="meta">"Código: MPL-2.0, © 2026 M.Hoffmann e os colaboradores do alerts."</p>
            </section>
        </article>
    }
}
