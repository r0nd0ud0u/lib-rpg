use std::collections::HashMap;

use crate::{attack_type::AttackType, equipment::Equipment, stats::Stats};

#[derive(Default, Debug, Clone)]
pub struct ExtendedCharacter {
    pub is_random_target: bool,
    pub is_heal_atk_blocked: bool,
    pub is_first_round: bool,
}

#[derive(Debug, Clone)]
pub struct Character {
    pub name: String,
    pub short_name: String,
    pub photo_name: String,
    pub stats: Stats,
    pub kind: CharacterType,
    pub level: u64,
    pub exp: u64,
    pub next_exp_level: u64,
    /// key: body, value: equipmentName
    pub equipment_on: HashMap<String, Equipment>,
    /// key: attak name, value: AttakType struct
    pub attacks_list: HashMap<String, AttackType>,
    /// That vector contains all the atks from m_AttakList and is sorted by level.
    pub attacks_by_lvl: Vec<AttackType>,
    pub selected_tier: Tier,
    pub color_theme: String,
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
            selected_tier: Tier::Standard,
            color_theme: "dark".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CharacterType {
    Hero,
    _Boss,
}

#[derive(Debug, Clone)]
pub enum Tier {
    Standard,
}
