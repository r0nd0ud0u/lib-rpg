#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum Rank {
    #[default]
    Common,
    Intermediate,
    Advanced,
}

impl Rank {
    pub fn to_str(&self) -> &str {
        match self {
            Rank::Common => "Common",
            Rank::Intermediate => "Intermediate",
            Rank::Advanced => "Advanced",
        }
    }
}
