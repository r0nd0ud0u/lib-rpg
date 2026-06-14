use std::str::FromStr;

use crate::common::constants::emoji_const::{
    EMOJI_BERSERK, EMOJI_HEALER, EMOJI_MAGE, EMOJI_WARRIOR,
};

/// Defines the class of the character
/// In the future, bonus and stats will be acquired.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Class {
    Standard,
    Berserker,
    Healer,
    Mage,
    Warrior,
}

impl FromStr for Class {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Standard" => Ok(Class::Standard),
            "Berserker" => Ok(Class::Berserker),
            "Healer" => Ok(Class::Healer),
            "Mage" => Ok(Class::Mage),
            "Warrior" => Ok(Class::Warrior),
            _ => Err(format!("Unknown class: {}", s)),
        }
    }
}

impl Class {
    pub fn to_str(&self) -> &str {
        match self {
            Class::Standard => "Standard",
            Class::Berserker => "Berserker",
            Class::Healer => "Healer",
            Class::Mage => "Mage",
            Class::Warrior => "Warrior",
        }
    }

    pub fn to_emoji(&self) -> &str {
        match self {
            Class::Standard => EMOJI_WARRIOR,
            Class::Berserker => EMOJI_BERSERK,
            Class::Healer => EMOJI_HEALER,
            Class::Mage => EMOJI_MAGE,
            Class::Warrior => EMOJI_WARRIOR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_from_str() {
        assert_eq!(Class::from_str("Standard").unwrap(), Class::Standard);
        assert_eq!(Class::from_str("Berserker").unwrap(), Class::Berserker);
        assert_eq!(Class::from_str("Healer").unwrap(), Class::Healer);
        assert_eq!(Class::from_str("Mage").unwrap(), Class::Mage);
        assert_eq!(Class::from_str("Warrior").unwrap(), Class::Warrior);
        assert!(Class::from_str("Unknown").is_err());
    }

    #[test]
    fn unit_to_str() {
        assert_eq!(Class::Standard.to_str(), "Standard");
        assert_eq!(Class::Berserker.to_str(), "Berserker");
        assert_eq!(Class::Healer.to_str(), "Healer");
        assert_eq!(Class::Mage.to_str(), "Mage");
        assert_eq!(Class::Warrior.to_str(), "Warrior");
    }

    #[test]
    fn unit_to_emoji() {
        assert!(!Class::Standard.to_emoji().is_empty());
        assert!(!Class::Berserker.to_emoji().is_empty());
        assert!(!Class::Healer.to_emoji().is_empty());
        assert!(!Class::Mage.to_emoji().is_empty());
        assert!(!Class::Warrior.to_emoji().is_empty());
    }
}
