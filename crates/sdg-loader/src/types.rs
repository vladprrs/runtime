use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root type for the Service Definition Graph (SDG v2).
///
/// Per D-04, the SDG has 4 top-level sections: `service`, `model`,
/// `computations`, and `api`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub schema_version: String,
    pub service: ServiceInfo,
    pub model: ModelDefinition,
    #[serde(default)]
    pub computations: ComputationsDefinition,
    #[serde(default)]
    pub api: ApiConfig,
}

/// Service identity metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_owner")]
    pub owner: String,
}

fn default_owner() -> String {
    "unknown".to_owned()
}

/// The model section containing aggregate definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    pub aggregates: HashMap<String, Aggregate>,
}

/// An aggregate root with fields, states, and transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregate {
    pub fields: HashMap<String, FieldDefinition>,
    pub states: Vec<String>,
    /// Per D-08: defaults to first element of `states` array.
    #[serde(default)]
    pub initial_state: Option<String>,
    #[serde(default)]
    pub transitions: HashMap<String, Transition>,
}

/// A field definition within an aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// The SDG type: "string", "integer", "float", "boolean", "uuid",
    /// "date", "datetime", "json", or array forms like "uuid[]".
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    // Validation properties per D-10
    #[serde(default)]
    pub min: Option<serde_json::Number>,
    #[serde(default)]
    pub max: Option<serde_json::Number>,
    #[serde(default)]
    pub min_length: Option<u64>,
    #[serde(default)]
    pub max_length: Option<u64>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    /// Per D-11: declarative relationship to another aggregate.
    #[serde(default)]
    pub references: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Per D-07: `from` accepts a single string or an array of strings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StateRef {
    Single(String),
    Multiple(Vec<String>),
}

/// A state transition within an aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: StateRef,
    /// Per D-06: `"$same"` sentinel allowed.
    pub to: String,
    #[serde(default)]
    pub command: Option<CommandDefinition>,
    /// Per D-21: references a computation node ID.
    #[serde(default)]
    pub guard: Option<String>,
    /// Per D-22: auto-populated event fields from computation outputs.
    #[serde(default)]
    pub auto_fields: HashMap<String, String>,
    /// Per D-23: override derived event name.
    #[serde(default)]
    pub event_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Command payload definition for a transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDefinition {
    #[serde(default)]
    pub fields: HashMap<String, FieldDefinition>,
}

/// The computations section: a flat DAG of typed function nodes.
///
/// Per D-12: nodes and edges are flat arrays. No nesting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComputationsDefinition {
    #[serde(default)]
    pub nodes: Vec<ComputationNode>,
    #[serde(default)]
    pub edges: Vec<Edge>,
}

/// A computation node in the DAG.
///
/// Per D-13: each node has a unique `id`, a `type` from the function catalog,
/// and type-specific `params`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationNode {
    pub id: String,
    /// Node type from the function catalog (field, command, context, literal,
    /// lookup, eq, neq, and, or, not, etc.).
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
}

/// An edge connecting computation nodes.
///
/// Per D-14: each edge has `from`, `to`, `port`, and optional `index`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub port: String,
    #[serde(default)]
    pub index: Option<u32>,
}

/// API configuration section.
///
/// Per D-38: parsed into typed Rust structs; semantic validation of
/// references deferred to Phase 6.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_expose")]
    pub expose: String,
    #[serde(default = "default_base_path")]
    pub base_path: String,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(default)]
    pub overrides: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub custom_queries: HashMap<String, CustomQuery>,
}

fn default_expose() -> String {
    "all".to_owned()
}

fn default_base_path() -> String {
    "/api".to_owned()
}

fn default_protocol() -> String {
    "http".to_owned()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            expose: default_expose(),
            base_path: default_base_path(),
            protocol: default_protocol(),
            overrides: HashMap::new(),
            custom_queries: HashMap::new(),
        }
    }
}

/// A custom query definition in the API section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomQuery {
    pub source: String,
    #[serde(default)]
    pub filter_by: Option<String>,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub group_by: Option<String>,
    #[serde(default)]
    pub aggregation: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Canonical task-tracker-extended.sdg.json fixture path relative to workspace root.
    fn canonical_fixture() -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        format!(
            "{manifest_dir}/../../specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json"
        )
    }

    #[test]
    fn test_deserialize_canonical_fixture() {
        let content =
            std::fs::read_to_string(canonical_fixture()).expect("canonical fixture must exist");
        let sd: ServiceDefinition =
            serde_json::from_str(&content).expect("canonical fixture must deserialize");
        assert_eq!(sd.service.name, "task-tracker-extended");
        assert_eq!(sd.model.aggregates.len(), 2);
        assert!(sd.model.aggregates.contains_key("User"));
        assert!(sd.model.aggregates.contains_key("Task"));
        let task = &sd.model.aggregates["Task"];
        assert_eq!(task.states.len(), 4);
    }

    #[test]
    fn test_deserialize_minimal_sdg() {
        let json = r#"{
            "schema_version": "2.0.0",
            "service": { "name": "test" },
            "model": { "aggregates": {} }
        }"#;
        let sd: ServiceDefinition =
            serde_json::from_str(json).expect("minimal SDG must deserialize");
        assert_eq!(sd.service.name, "test");
        assert_eq!(sd.service.owner, "unknown");
        assert!(sd.model.aggregates.is_empty());
        assert!(sd.computations.nodes.is_empty());
        assert_eq!(sd.api.expose, "all");
    }

    #[test]
    fn test_state_ref_single() {
        let sr: StateRef = serde_json::from_str(r#""Created""#).unwrap();
        assert_eq!(sr, StateRef::Single("Created".to_owned()));
    }

    #[test]
    fn test_state_ref_multiple() {
        let sr: StateRef = serde_json::from_str(r#"["Created", "InProgress"]"#).unwrap();
        assert_eq!(
            sr,
            StateRef::Multiple(vec!["Created".to_owned(), "InProgress".to_owned()])
        );
    }
}
