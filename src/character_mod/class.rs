use crate::common::constants::emoji_const::{EMOJI_HEALER, EMOJI_MAGE, EMOJI_TANK, EMOJI_WARRIOR};

/// Defines the class of the character
/// In the future, bonus and stats will be acquired.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Class {
    Standard,
    Tank,
    Healer,
    Mage,
}

impl Class {
    pub fn from_str(class: &str) -> Self {
        match class {
            "Standard" => Class::Standard,
            "Tank" => Class::Tank,
            "Healer" => Class::Healer,
            "Mage" => Class::Mage,
            _ => panic!("Unknown class: {}", class),
        }
    }

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
