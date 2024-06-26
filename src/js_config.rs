use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A representation of the NodeJS `sea-config.json` configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SEAConfig {
    /// The main entrypoint to the file that is to be bundled.
    pub main: String,

    /// The output SEA blob name.
    pub output: String,

    // Any other fields that are not explicitly defined.
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

/// A representation of the NodeJS `package.json` configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    /// The name of the project.
    pub name: String,

    /// The main entrypoint as defined by the project.
    pub main: Option<String>,

    // Any other fields that are not explicitly defined.
    #[serde(flatten)]
    other: HashMap<String, Value>,
}
