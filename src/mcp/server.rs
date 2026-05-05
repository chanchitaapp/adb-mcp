use super::protocol::*;
use log::{debug, error, info};
use serde_json::{json, Value};
use std::collections::HashMap;

type ToolHandler = Box<dyn Fn(Value) -> std::result::Result<Value, String> + Send>;

pub struct McpServer {
    name: String,
    version: String,
    tools: HashMap<String, (ToolDefinition, ToolHandler)>,
}

impl McpServer {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            tools: HashMap::new(),
        }
    }

    pub fn register_tool<F>(
        mut self,
        name: &str,
        description: &str,
        input_schema: Value,
        handler: F,
    ) -> Self
    where
        F: Fn(Value) -> std::result::Result<Value, String> + Send + 'static,
    {
        let definition = ToolDefinition {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        };

        self.tools
            .insert(name.to_string(), (definition, Box::new(handler)));

        info!("Registered tool: {}", name);
        self
    }

    pub async fn handle_message(&self, message: String) -> Option<String> {
        match serde_json::from_str::<JsonRpcRequest>(&message) {
            Ok(request) => {
                let response = self.handle_request(&request).await?;
                Some(serde_json::to_string(&response).unwrap_or_else(|_| {
                    let id = request.id.unwrap_or(Value::Null);
                    serde_json::to_string(&JsonRpcResponse::internal_error(
                        id,
                        "Failed to serialize response".to_string(),
                    ))
                    .unwrap()
                }))
            }
            Err(e) => {
                error!("Failed to parse JSON-RPC request: {}", e);
                Some(
                    serde_json::to_string(&json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32700,
                            "message": "Parse error"
                        },
                        "id": Value::Null
                    }))
                    .unwrap(),
                )
            }
        }
    }

    async fn handle_request(&self, request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        let Some(id) = request.id.clone() else {
            self.handle_notification(request);
            return None;
        };

        debug!("Handling request: {} (id: {})", request.method, id);

        let response = match request.method.as_str() {
            "initialize" => self.handle_initialize(id),
            "tools/list" => self.handle_list_tools(id),
            "tools/call" => self.handle_call_tool(id, request).await,
            _ => {
                debug!("Unknown method: {}", request.method);
                JsonRpcResponse::method_not_found(id)
            }
        };

        Some(response)
    }

    fn handle_notification(&self, request: &JsonRpcRequest) {
        match request.method.as_str() {
            "notifications/initialized" => info!("Client initialization complete"),
            _ => debug!("Ignoring notification: {}", request.method),
        }
    }

    fn handle_initialize(&self, id: Value) -> JsonRpcResponse {
        info!("Client initialized");

        let response = InitializeResponse {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(json!({ "list_changed": true })),
                resources: None,
                prompts: None,
            },
            server_info: ServerInfo {
                name: self.name.clone(),
                version: self.version.clone(),
            },
        };

        JsonRpcResponse::success(id, serde_json::to_value(response).unwrap())
    }

    fn handle_list_tools(&self, id: Value) -> JsonRpcResponse {
        let tools: Vec<ToolDefinition> = self.tools.values().map(|(def, _)| def.clone()).collect();

        let response = ListToolsResponse { tools };

        JsonRpcResponse::success(id, serde_json::to_value(response).unwrap())
    }

    async fn handle_call_tool(&self, id: Value, request: &JsonRpcRequest) -> JsonRpcResponse {
        let call_req: Result<CallToolRequest, _> = serde_json::from_value(request.params.clone());

        match call_req {
            Ok(call) => {
                if let Some((_, handler)) = self.tools.get(&call.name) {
                    match handler(call.arguments) {
                        Ok(result) => {
                            let content = match result {
                                Value::String(s) => vec![ToolContent::text(s)],
                                other => vec![ToolContent::text(other.to_string())],
                            };

                            let response = CallToolResponse {
                                content,
                                is_error: None,
                            };

                            JsonRpcResponse::success(id, serde_json::to_value(response).unwrap())
                        }
                        Err(e) => {
                            error!("Tool execution failed: {}", e);
                            let response = CallToolResponse {
                                content: vec![ToolContent::text(format!("Error: {}", e))],
                                is_error: Some(true),
                            };

                            JsonRpcResponse::success(id, serde_json::to_value(response).unwrap())
                        }
                    }
                } else {
                    JsonRpcResponse::invalid_params(id, format!("Tool not found: {}", call.name))
                }
            }
            Err(e) => {
                error!("Invalid tool call parameters: {}", e);
                JsonRpcResponse::invalid_params(id, e.to_string())
            }
        }
    }
}

pub struct InputSchema {
    pub properties: HashMap<String, PropertySchema>,
    pub required: Vec<String>,
}

pub struct PropertySchema {
    pub schema_type: String,
    pub description: String,
}

impl InputSchema {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            required: Vec::new(),
        }
    }

    pub fn add_property(
        mut self,
        name: &str,
        schema_type: &str,
        description: &str,
        required: bool,
    ) -> Self {
        self.properties.insert(
            name.to_string(),
            PropertySchema {
                schema_type: schema_type.to_string(),
                description: description.to_string(),
            },
        );

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    pub fn to_json(&self) -> Value {
        let mut properties = serde_json::Map::new();

        for (name, schema) in &self.properties {
            properties.insert(
                name.clone(),
                json!({
                    "type": schema.schema_type,
                    "description": schema.description,
                }),
            );
        }

        json!({
            "type": "object",
            "properties": properties,
            "required": self.required,
        })
    }
}

impl Default for InputSchema {
    fn default() -> Self {
        Self::new()
    }
}
