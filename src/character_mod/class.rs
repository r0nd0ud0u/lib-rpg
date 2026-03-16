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
