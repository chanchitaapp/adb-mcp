use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod protocol;
pub mod server;
pub mod cursor;

pub use protocol::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse};
pub use server::McpServer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}
