#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EndOfScenario {
    pub scenario_level: u64,
    pub characters_levelup: Vec<LevelUp>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LevelUp {
    pub character_id_name: String,
    pub new_level: u64,
    pub old_level: u64,
}

impl EndOfScenario {
    pub fn to_formatted_string(&self) -> String {
        let mut result = format!("Scenario Level: {}\n", self.scenario_level);
        for level_up in &self.characters_levelup {
            if level_up.new_level > level_up.old_level {
                result.push_str(&format!(
                    "Character {} UP: {} to {} \n",
                    level_up.character_id_name, level_up.old_level, level_up.new_level
                ));
            } else {
                result.push_str(&format!(
                    "Character {} =: {} \n",
                    level_up.character_id_name, level_up.old_level
                ));
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn unit_end_of_scenario_to_formatted_string() {
        let end_of_scenario = EndOfScenario {
            scenario_level: 5,
            characters_levelup: vec![
                LevelUp {
                    character_id_name: "Hero1".to_string(),
                    new_level: 3,
                    old_level: 2,
                },
                LevelUp {
                    character_id_name: "Hero2".to_string(),
                    new_level: 2,
                    old_level: 2,
                },
            ],
        };
        let formatted_string = end_of_scenario.to_formatted_string();
        let expected_string =
            "Scenario Level: 5\nCharacter Hero1 UP: 2 to 3 \nCharacter Hero2 =: 2 \n";
        assert_eq!(formatted_string, expected_string);
    }
}
