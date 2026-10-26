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
