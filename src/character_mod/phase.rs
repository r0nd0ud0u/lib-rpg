#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Phase {
    pub name: String,
    pub rank: Rank,
    pub pattern: Vec<String>,
}