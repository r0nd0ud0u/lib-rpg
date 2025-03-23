use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

use crate::{
    attack_type::AttackType,
    buffers::Buffers,
    equipment::Equipment,
    powers::Powers,
    stats::{Stats, TxRx},
    utils,
};

/// ExtendedCharacter
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct ExtendedCharacter {
    /// Fight information: Is the random character targeted by the current attack of other character
    #[serde(default, rename = "is_random_target")]
    pub is_random_target: bool,
    /// Fight information: TODO is_heal_atk_blocked
    #[serde(default, rename = "is_heal_atk_blocked")]
    pub is_heal_atk_blocked: bool,
    /// Fight information: Playing the first round of that tour
    #[serde(default, rename = "is_first_round")]
    pub is_first_round: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Character {
    /// Full Name of the character
    #[serde(rename = "Name")]
    pub name: String,
    /// Short name of the character
    #[serde(rename = "Short name")]
    pub short_name: String,
    /// Name of the photo of the character without extension
    #[serde(rename = "Photo")]
    pub photo_name: String,
    /// Stats about all the capacities and current state
    #[serde(rename = "Stats")]
    pub stats: Stats,
    /// Type of the character {Hero, Boss}
    #[serde(rename = "Type")]
    pub kind: CharacterType,
    /// Class of the character {Standard, Tank ...}
    #[serde(rename = "Class")]
    pub class: Class,
    /// Level of the character, start 1
    #[serde(rename = "Level")]
    pub level: u64,
    /// Experience of the character, start 0
    #[serde(rename = "Experience")]
    pub exp: u64,
    /// Experience to acquire to upgrade to next level
    pub next_exp_level: u64,
    /// key: body, value: equipmentName
    pub equipment_on: HashMap<String, Equipment>,
    /// key: attak name, value: AttakType struct
    pub attacks_list: HashMap<String, AttackType>,
    /// That vector contains all the atks from m_AttakList and is sorted by level.
    pub attacks_by_lvl: Vec<AttackType>,
    /// Main color theme of the character
    #[serde(rename = "Color")]
    pub color_theme: String,
    /// Fight information: last attack was critical
    pub is_last_atk_crit: bool,
    /// Fight information: damages transmitted or received through the fight
    #[serde(rename = "Tx-rx")]
    tx_rx: Vec<TxRx>,
    /// Fight information: Enabled buf/debuf acquired through the fight
    #[serde(rename = "Buf-debuf")]
    pub all_buffers: Vec<Buffers>,
    /// Powers
    #[serde(rename = "Powers")]
    pub power: Powers,
    /// ExtendedCharacter
    #[serde(rename = "ExtendedCharacter")]
    pub extended_character: ExtendedCharacter,
    /// Fight information: attack can be blocked
    #[serde(rename = "is-blocking-atk")]
    pub is_blocking_atk: bool,
    /// Fight information: nb-actions-in-round
    #[serde(rename = "nb-actions-in-round")]
    pub actions_done_in_round: u64,
    /// Fight information: max-actions-in-round
    #[serde(rename = "max-actions-by-round")]
    pub max_actions_by_round: u64,
    /// TODO rank
    #[serde(rename = "Rank")]
    pub rank: u64,
    /// TODO shape
    #[serde(rename = "Shape")]
    pub shape: String,
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
            rank: 0,
            shape: String::new(),
        }
    }
}

/// Defines the type of player: hero -> player, boss -> computer.
/// "PascalCase" ensures that "Hero" and "Boss" from JSON map correctly to the Rust enum variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum CharacterType {
    Hero,
    Boss,
}

/// Defines the class of the character
/// In the future, bonus and stats will be acquired.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Class {
    Standard,
    Tank,
}

impl Character {
    pub fn try_new_from_json<P: AsRef<Path>>(path: P) -> Result<Character> {
        if let Ok(mut value) = utils::read_from_json::<_, Character>(&path) {
            value.stats.sync_raw_values();
            Ok(value)
        } else {
            Err(anyhow!("Unknown file: {:?}", path.as_ref()))
        }
    }

    pub fn is_dead(&self) -> bool {
        self.stats.hp.current == 0
    }
}

#[cfg(test)]
mod tests {
    use crate::character::{CharacterType, Class};

    use super::Character;

    #[test]
    fn unit_try_new_from_json() {
        let file_path = "./tests/characters/test.json"; // Path to the JSON file
        let c = Character::try_new_from_json(file_path).unwrap();
        // name
        assert_eq!("Super test", c.name);
        assert_eq!("Test", c.short_name);
        // buf-debuf
        assert_eq!(12, c.all_buffers.len());
        assert_eq!("hp,mana", c.all_buffers[0].all_stat_name);
        assert_eq!(3, c.all_buffers[0].buf_type);
        assert_eq!(false, c.all_buffers[0].is_passive_enabled);
        assert_eq!(true, c.all_buffers[0].is_percent);
        assert_eq!(100, c.all_buffers[0].value);
        // Class
        assert_eq!(Class::Standard, c.class);
        // Color
        assert_eq!("green", c.color_theme);
        // Experience
        assert_eq!(50, c.exp);
        // extended character
        assert_eq!(false, c.extended_character.is_first_round);
        assert_eq!(true, c.extended_character.is_heal_atk_blocked);
        assert_eq!(false, c.extended_character.is_random_target);
        // level
        assert_eq!(1, c.level);
        // photo
        assert_eq!("phototest", c.photo_name);
        // powers
        assert_eq!(false, c.power.is_crit_heal_after_crit);
        assert_eq!(true, c.power.is_damage_tx_heal_needy_ally);
        // rank
        assert_eq!(4, c.rank);
        // shape
        assert_eq!("", c.shape);
        // stats
        // stats - aggro
        assert_eq!(0, c.stats.aggro.current);
        assert_eq!(9999, c.stats.aggro.max);
        // stats - aggro rate
        assert_eq!(1, c.stats.aggro_rate.current);
        assert_eq!(1, c.stats.aggro_rate.max);
        // stats - berseck
        assert_eq!(200, c.stats.berseck.current);
        assert_eq!(200, c.stats.berseck.max);
        // stats - berseck_rate
        assert_eq!(1, c.stats.berseck_rate.current);
        assert_eq!(1, c.stats.berseck_rate.max);
        // stats - critical_strike
        assert_eq!(10, c.stats.critical_strike.current);
        assert_eq!(10, c.stats.critical_strike.max);
        // stats - dodge
        assert_eq!(5, c.stats.dodge.current);
        assert_eq!(5, c.stats.dodge.max);
        // stats - hp
        assert_eq!(1, c.stats.hp.current);
        assert_eq!(135, c.stats.hp.max);
        assert_eq!(135, c.stats.hp.max_raw);
        assert_eq!(1, c.stats.hp.current_raw);
        // stats - hp_regeneration
        assert_eq!(7, c.stats.hp_regeneration.current);
        assert_eq!(7, c.stats.hp_regeneration.max);
        // stats - magic_armor
        assert_eq!(10, c.stats.magical_armor.current);
        assert_eq!(10, c.stats.magical_armor.max);
        // stats - magic_power
        assert_eq!(20, c.stats.magic_power.current);
        assert_eq!(20, c.stats.magic_power.max);
        // stats - mana
        assert_eq!(200, c.stats.mana.current);
        assert_eq!(200, c.stats.mana.max);
        // stats - mana_regeneration
        assert_eq!(7, c.stats.mana_regeneration.current);
        assert_eq!(7, c.stats.mana_regeneration.max);
        // stats - physical_armor
        assert_eq!(5, c.stats.physical_armor.current);
        assert_eq!(5, c.stats.physical_armor.max);
        // stats - physical_power
        assert_eq!(10, c.stats.physical_power.current);
        assert_eq!(10, c.stats.physical_power.max);
        // stats - speed
        assert_eq!(212, c.stats.speed.current);
        assert_eq!(212, c.stats.speed.max);
        // stats - speed_regeneration
        assert_eq!(12, c.stats.speed_regeneration.current);
        assert_eq!(12, c.stats.speed_regeneration.max);
        // stats - vigor
        assert_eq!(200, c.stats.vigor.current);
        assert_eq!(200, c.stats.vigor.max);
        // stats - vigor_regeneration
        assert_eq!(5, c.stats.vigor_regeneration.current);
        assert_eq!(5, c.stats.vigor_regeneration.max);
        // tx-rx
        assert_eq!(6, c.tx_rx.len());
        assert_eq!(0, c.tx_rx[2].tx_rx_size);
        assert_eq!(2, c.tx_rx[2].tx_rx_type);
        // Type - kind
        assert_eq!(CharacterType::Hero, c.kind);
        // is-blocking-atk
        assert_eq!(false, c.is_blocking_atk);
        // max_actions_by_round
        assert_eq!(1, c.max_actions_by_round);
        // nb-actions-in-round
        assert_eq!(0, c.actions_done_in_round);

        let file_path = "./tests/characters/wrong.json";
        assert!(Character::try_new_from_json(file_path).is_err());
    }
}
