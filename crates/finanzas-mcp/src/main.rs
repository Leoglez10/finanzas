use std::sync::Mutex;

use finanzas_core::store::Store;
use finanzas_mcp::tools;
use rmcp::{
    model::{CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool, ToolAnnotations},
    service::RequestContext,
    ErrorData, RoleServer, ServerHandler, ServiceExt,
};
use serde_json::Value;

struct Finanzas { store: Mutex<Store> }

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
        let result = match self.store.lock() {
            Ok(mut store) => tools::call(&mut store, &request.name, &Value::Object(request.arguments.unwrap_or_default()), chrono::Local::now().date_naive())
                .and_then(|value| serde_json::to_string_pretty(&value).map_err(|e| tools::ToolError(e.to_string()))),
            Err(e) => Err(tools::ToolError(e.to_string())),
        };
        Ok(match result {
            Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(format!("Error: {error}"))]),
        }.into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Store::default_path().ok_or("Unable to determine database path")?;
    let server = Finanzas { store: Mutex::new(Store::open(path)?) };
    server.serve(rmcp::transport::stdio()).await?.waiting().await?;
    Ok(())
}
