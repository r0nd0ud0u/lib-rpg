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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_to_str() {
        assert_eq!(Rank::Common.to_str(), "Common");
        assert_eq!(Rank::Intermediate.to_str(), "Intermediate");
        assert_eq!(Rank::Advanced.to_str(), "Advanced");
    }
}
