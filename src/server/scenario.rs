use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    /// Boss patterns, used for boss to adapt the behavior of the fight
    /// The key is the name of the boss, and the value is a list of pattern indexes that the boss can use
    pub boss_patterns: HashMap<String, Vec<u64>>,
}
