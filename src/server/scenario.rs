use std::collections::HashMap;

use anyhow::{Result, bail};

use crate::{character_mod::loot::Loot, utils};

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    /// Boss patterns, used for boss to adapt the behavior of the fight
    /// The key is the name of the boss, and the value is a list of pattern indexes that the boss can use
    pub boss_patterns: HashMap<String, Vec<u64>>,
    /// Loots to give to the heroes at the end of the scenario, if they win
    #[serde(default)]
    pub loots: Vec<Loot>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ScenarioState {
    #[default]
    NotStarted = 0,
    InProgress,
    Completed,
}

impl Scenario {
    pub fn try_new_from_json<P: AsRef<std::path::Path>>(path: P) -> Result<Scenario> {
        if let Ok(value) = utils::read_from_json::<_, Scenario>(&path) {
            // check if the scenario is valid
            if value.name.is_empty() {
                bail!("Scenario name is empty in file: {:?}", path.as_ref());
            }
            if value.description.is_empty() {
                bail!("Scenario description is empty in file: {:?}", path.as_ref());
            }
            Ok(value)
        } else {
            bail!("Failed to read scenario from file: {:?}", path.as_ref());
        }
    }
}
