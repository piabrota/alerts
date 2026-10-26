//// The alerts page at piabrota.art/alerts/: one module, no Hex dependencies, so it builds in
//// the Nix sandbox. `gleam run` prints the page; site/justfile writes it to index.html. Every
//// link is relative, so the page works under any path.

@external(javascript, "./site_ffi.mjs", "log")
fn log(line: String) -> Nil

fn item(title: String, text: String) -> String {
  "<li><strong>" <> title <> "</strong> " <> text <> "</li>"
}

fn link(href: String, text: String) -> String {
  "<a href=\"" <> href <> "\">" <> text <> "</a>"
}

pub fn page() -> String {
  "<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\">"
  <> "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
  <> "<title>alerts</title><link rel=\"stylesheet\" href=\"style.css\"></head><body><main>"
  <> "<img class=\"wave\" src=\"wave.svg\" alt=\"Uma onda de Fibonacci e um sino, de @h0ffmann\">"
  <> "<h1>alerts</h1><p class=\"lead\">Alertas do século 22.</p>"
  <> "<ul>"
  <> item("Plataforma de alertas genérica.", "O alerts coleta alertas de fontes oficiais, filtra os alertas e entrega os alertas.")
  <> item("Só fontes oficiais.", "O alerts nunca inventa um alerta, uma área, um horário ou uma ação que a fonte não diz.")
  <> item("Primeiro risco: inundação no Brasil.", "A v0 atende a Região Sul: PR, SC e RS.")
  <> item("Dois canais, o mesmo texto.", "Discord e um feed Atom com alertas CAP 1.2.")
  <> item("Estado: bootstrap.", "Nenhuma fonte está implementada.")
  <> "</ul>"
  <> "<img class=\"figure\" src=\"fluxo-alerta.png\" alt=\"Como um alerta oficial chega ao Discord e ao feed\">"
  <> "<nav>"
  <> link("https://github.com/piabrota/alerts", "código")
  <> link("grafo/graph.html", "grafo do código")
  <> link("https://github.com/piabrota/alerts/wiki/Home-pt-BR", "wiki")
  <> "</nav>"
  <> "<footer>MPL-2.0 · desenho de " <> link("https://github.com/h0ffmann", "@h0ffmann") <> "</footer>"
  <> "</main></body></html>"
}

pub fn main() -> Nil {
  log(page())
}
