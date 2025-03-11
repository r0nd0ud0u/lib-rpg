use std::{collections::HashMap, fs::File, io::Read};

use serde::{Deserialize, Serialize};

use crate::{
    attack_type::AttackType, buffers::Buffers, equipment::Equipment, powers::Powers, stats::Stats,
    stats::TxRx,
};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ExtendedCharacter {
    #[serde(default, rename = "is_random_target")]
    pub is_random_target: bool,
    #[serde(default, rename = "is_heal_atk_blocked")]
    pub is_heal_atk_blocked: bool,
    #[serde(default, rename = "is_first_round")]
    pub is_first_round: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Character {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Short name")]
    pub short_name: String,
    #[serde(rename = "Photo")]
    pub photo_name: String,
    #[serde(rename = "Stats")]
    pub stats: Stats,
    #[serde(rename = "Type")]
    pub kind: CharacterType,
    #[serde(rename = "Class")]
    pub class: Class,
    #[serde(rename = "Level")]
    pub level: u64,
    #[serde(rename = "Experience")]
    pub exp: u64,
    pub next_exp_level: u64,
    /// key: body, value: equipmentName
    pub equipment_on: HashMap<String, Equipment>,
    /// key: attak name, value: AttakType struct
    pub attacks_list: HashMap<String, AttackType>,
    /// That vector contains all the atks from m_AttakList and is sorted by level.
    pub attacks_by_lvl: Vec<AttackType>,
    #[serde(rename = "Color")]
    pub color_theme: String,
    pub is_last_atk_crit: bool,
    #[serde(rename = "Tx-rx")]
    tx_rx: Vec<TxRx>,
    #[serde(rename = "Buf-debuf")]
    pub all_buffers: Vec<Buffers>,
    #[serde(rename = "Powers")]
    pub power: Powers,
    #[serde(rename = "ExtendedCharacter")]
    pub extended_character: ExtendedCharacter,
    #[serde(rename = "is-blocking-atk")]
    pub is_blocking_atk: bool,
    #[serde(rename = "nb-actions-in-round")]
    pub actions_done_in_round: u64,
    #[serde(rename = "max-nb-actions-in-round")]
    pub max_actions_by_round: u64,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            name: String::from("default"),
            short_name: String::from("default"),
            photo_name: String::from("default"),
            stats: Stats::default(),
            kind: CharacterType::Hero,
            equipment_on: HashMap::new(),
            attacks_list: HashMap::new(),
            level: 1,
            exp: 0,
            next_exp_level: 100,
            attacks_by_lvl: vec![],
            color_theme: "dark".to_owned(),
            is_last_atk_crit: false,
            all_buffers: vec![],
            is_blocking_atk: false,
            power: Powers::default(),
            extended_character: ExtendedCharacter::default(),
            actions_done_in_round: 0,
            max_actions_by_round: 0,
            class: Class::Standard,
            tx_rx: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CharacterType {
    Hero,
    Boss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Class {
    Standard,
    Tank,
}

impl Character {
    pub fn decode_json(file_path: &str) -> Result<Character, serde_json::Error> {
        // Open the file
        let mut file = File::open(file_path).expect("File not found");
        // Read the file content into a string
        let mut json_str = String::new();
        file.read_to_string(&mut json_str)
            .expect("Failed to read file");
        // Deserialize the JSON string into a Character struct
        serde_json::from_str(&json_str)
    }
}

#[cfg(test)]
mod tests {
    use super::Character;

    #[test]
    fn unit_decode_json() {
        let file_path = "./tests/characters/test.json"; // Path to the JSON file
        match Character::decode_json(file_path) {
            Ok(character) => {
                println!("Decoded character: {:?}", character);
            }
            Err(e) => {
                println!("Error decoding JSON: {}", e);
            }
        }
    }
}
