//! MCP server (stdio) that exposes alerts to agents.

use alerts_core::{Alert, Harness, Uf, only_ufs};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};

#[derive(Debug, Clone)]
struct Alerts {
    #[expect(dead_code, reason = "tool_handler macro accesses this router field")]
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CheckAlert {
    #[schemars(description = "one alerts Alert, as JSON")]
    alert_json: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SearchSources {
    #[schemars(
        description = "words that must all appear (agency, hazard, format...), any case or accent"
    )]
    #[serde(default)]
    text: String,
    #[schemars(
        description = "an ISO 3166-1 alpha-2 country code, for example BR; world sources always match"
    )]
    country: Option<String>,
    #[schemars(description = "only the sources that need no token (BYOT)")]
    #[serde(default)]
    no_token: bool,
    #[schemars(description = "only the sources that give alerts, not observations")]
    #[serde(default)]
    alerts_only: bool,
}

#[tool_router]
impl Alerts {
    #[tool(description = "List the Brazilian states (UF) that alerts v0 serves")]
    fn v0_ufs(&self) -> String {
        Uf::V0
            .iter()
            .map(|u| u.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[tool(
        description = "Search the curated catalog of official alert sources, in Brazil and in the world. \
                       Each result gives the endpoint, the format, and the environment variable of the token \
                       when the source needs your own token (BYOT). Never returns a token."
    )]
    fn search_sources(&self, Parameters(q): Parameters<SearchSources>) -> String {
        let found = alerts_catalog::search(&alerts_catalog::Query {
            text: q.text,
            country: q.country,
            no_token: q.no_token,
            alerts_only: q.alerts_only,
        });
        let rows: Vec<_> = found
            .iter()
            .map(|e| {
                let (token_env, token_how) = match e.auth {
                    alerts_catalog::Auth::None => (None, None),
                    alerts_catalog::Auth::Token { env, how } => (Some(env), Some(how)),
                };
                serde_json::json!({
                    "id": e.id,
                    "name": e.name,
                    "agency": e.agency,
                    "countries": e.countries,
                    "hazards": e.hazards,
                    "kind": match e.kind {
                        alerts_catalog::Kind::Alerts => "alerts",
                        alerts_catalog::Kind::Observations => "observations",
                    },
                    "format": e.format,
                    "url": e.url,
                    "docs": e.docs,
                    "token_env": token_env,
                    "token_how": token_how,
                    "module": e.module,
                    "checked": e.checked,
                })
            })
            .collect();
        serde_json::Value::Array(rows).to_string()
    }

    #[tool(description = "Say whether the v0 harness keeps or drops an alert")]
    fn check_alert(&self, Parameters(CheckAlert { alert_json }): Parameters<CheckAlert>) -> String {
        match serde_json::from_str::<Alert>(&alert_json) {
            Ok(a) => match v0_harness().run(a) {
                Some(_) => "kept".into(),
                None => "dropped".into(),
            },
            Err(e) => format!("invalid alert: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for Alerts {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
}

fn v0_harness() -> Harness {
    Harness::new().then(only_ufs(Uf::V0.to_vec()))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Alerts {
        tool_router: Alerts::tool_router(),
    };
    server
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
