use std::str::FromStr;

use crate::common::constants::emoji_const::{EMOJI_HEALER, EMOJI_MAGE, EMOJI_TANK, EMOJI_WARRIOR};

/// Defines the class of the character
/// In the future, bonus and stats will be acquired.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Class {
    Standard,
    Tank,
    Healer,
    Mage,
}

impl FromStr for Class {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Standard" => Ok(Class::Standard),
            "Tank" => Ok(Class::Tank),
            "Healer" => Ok(Class::Healer),
            "Mage" => Ok(Class::Mage),
            _ => Err(format!("Unknown class: {}", s)),
        }
    }
}

impl Class {
    pub fn to_str(&self) -> &str {
        match self {
            Class::Standard => "Standard",
            Class::Tank => "Tank",
            Class::Healer => "Healer",
            Class::Mage => "Mage",
        }
    }

    pub fn to_emoji(&self) -> &str {
        match self {
            Class::Standard => EMOJI_WARRIOR,
            Class::Tank => EMOJI_TANK,
            Class::Healer => EMOJI_HEALER,
            Class::Mage => EMOJI_MAGE,
        }
    }
}
