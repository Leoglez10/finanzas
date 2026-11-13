use std::{collections::HashMap, sync::Mutex};

use finanzas_core::store::Store;
use finanzas_mcp::tools;
use rmcp::{
    model::{CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool, ToolAnnotations},
    service::RequestContext,
    ErrorData, RoleServer, ServerHandler, ServiceExt,
};
use serde_json::Value;

struct Finanzas { stores: Mutex<HashMap<String, Store>> }

impl ServerHandler for Finanzas {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("finanzas", "1.0.0"))
    }

    async fn list_tools(&self, _: Option<PaginatedRequestParams>, _: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        let tools = tools::specs().into_iter().map(|spec| {
            let schema = spec.input_schema.as_object().expect("tool schema is an object").clone();
            Tool::new(spec.name, spec.description, schema)
                .with_annotations(ToolAnnotations::new().read_only(spec.read_only).destructive(spec.destructive))
        }).collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(&self, request: CallToolRequestParams, _: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        let args = Value::Object(request.arguments.unwrap_or_default());
        let result = tools::profile(&args).and_then(|profile| {
            let mut stores = self.stores.lock().map_err(|e| tools::ToolError(e.to_string()))?;
            // Profiles are created, renamed and deleted in the app; only an existing one is opened.
            let path = Store::profile_path(&profile).filter(|path| profile == "personal" || path.exists());
            let Some(path) = path else { stores.remove(&profile); return Err(tools::ToolError(format!("perfil: unknown profile {profile}"))) };
            if !stores.contains_key(&profile) { stores.insert(profile.clone(), Store::open(path).map_err(|e| tools::ToolError(e.to_string()))?); }
            let store = stores.get_mut(&profile).expect("store opened above");
            tools::call(store, &request.name, &args, chrono::Local::now().date_naive())
        }).and_then(|value| serde_json::to_string_pretty(&value).map_err(|e| tools::ToolError(e.to_string())));
        Ok(match result {
            Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(format!("Error: {error}"))]),
        }.into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Store::default_path().ok_or("Unable to determine database path")?;
    let dir = finanzas_core::agents::heartbeat_dir(&path);
    let server = Finanzas { stores: Mutex::new(HashMap::from([("personal".to_owned(), Store::open(path)?)])) };
    let running = server.serve(rmcp::transport::stdio()).await?;
    // Known clients mark themselves connected for the app's Ajustes screen.
    let agent = running.peer().peer_info().and_then(|info| finanzas_core::agents::Agent::from_client(&info.client_info.name));
    if let Some(agent) = agent {
        let _ = finanzas_core::agents::beat(&dir, agent);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(finanzas_core::agents::BEAT).await;
                let _ = finanzas_core::agents::beat(&dir, agent);
            }
        });
    }
    running.waiting().await?;
    Ok(())
}
