use std::collections::HashMap;

use anyhow::{Result, bail};

use crate::{character_mod::loot::Loot, utils};

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    /// Name of the scenario, used to identify it and for players to choose it
    pub name: String,
    /// Description of the scenario, used to explain the story and the objectives to the players
    pub description: String,
    /// Boss patterns, used for boss to adapt the behavior of the fight
    /// The key is the name of the boss, and the value is a list of pattern indexes that the boss can use
    pub boss_patterns: HashMap<String, Vec<u64>>,
    /// Loots to give to the heroes at the end of the scenario, if they win
    #[serde(default)]
    pub loots: Vec<Loot>,
    /// Level of the scenario, used to know the difficulty and to adapt the rewards
    pub level: u64,
    /// Universe/theme the scenario belongs to (e.g. "lotr", "pokemon").
    /// Empty string means the scenario is in the default universe.
    #[serde(default)]
    pub universe: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_scenario(content: &str) -> std::path::PathBuf {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "scenario_test_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        tmp
    }

    #[test]
    fn unit_try_new_from_json_not_found() {
        let result = Scenario::try_new_from_json("/nonexistent/path/scenario.json");
        assert!(result.is_err());
    }

    #[test]
    fn unit_try_new_from_json_empty_name() {
        let path = write_temp_scenario(r#"{"name":"","description":"Some desc","level":1}"#);
        let result = Scenario::try_new_from_json(&path);
        assert!(result.is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unit_try_new_from_json_empty_description() {
        let path = write_temp_scenario(r#"{"name":"Stage 1","description":"","level":1}"#);
        let result = Scenario::try_new_from_json(&path);
        assert!(result.is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unit_try_new_from_json_valid() {
        let path = write_temp_scenario(
            r#"{"name":"Stage 1","description":"A test stage","level":1,"boss_patterns":{}}"#,
        );
        let result = Scenario::try_new_from_json(&path);
        assert!(result.is_ok());
        let s = result.unwrap();
        assert_eq!(s.name, "Stage 1");
        assert_eq!(s.level, 1);
        let _ = std::fs::remove_file(path);
    }
}
