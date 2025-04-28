use std::collections::HashMap;

/// Shared state that can be passed between nodes
pub type SharedState = serde_json::Value;
/// Parameters that can be configured for nodes
pub type Params = HashMap<String, serde_json::Value>;
/// Action string used for determining the next node in a flow
pub type ActionKey = String;
pub const DEFAULT_ACTION: &str = "default";
