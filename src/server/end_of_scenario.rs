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
