//// The home of piabrota.art (`/index.html`): one page that points to the projects under it.
//// `gleam run -m home` prints it; site/justfile writes it to serve/index.html.

@external(javascript, "./site_ffi.mjs", "log")
fn log(line: String) -> Nil

pub fn page() -> String {
  "<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\">"
  <> "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
  <> "<title>piabrota</title><link rel=\"stylesheet\" href=\"alerts/style.css\"></head><body><main>"
  <> "<h1>piabrota</h1>"
  <> "<ul><li><a href=\"alerts/\">ialerts</a>: alertas oficiais de inundação no Sul do Brasil.</li></ul>"
  <> "</main></body></html>"
}

pub fn main() -> Nil {
  log(page())
}
