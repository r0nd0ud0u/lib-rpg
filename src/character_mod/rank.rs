#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum Rank {
    #[default]
    Beginner,
    Intermediate,
    Advanced,
}