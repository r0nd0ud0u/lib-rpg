#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum Rank {
    #[default]
    Beginner,
    Intermediate,
    Advanced,
}