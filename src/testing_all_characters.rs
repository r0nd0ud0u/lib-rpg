#![allow(dead_code)]

use std::collections::HashMap;

use crate::character::Character;
use crate::equipment::{Equipment, EquipmentJsonKey};
use crate::players_manager::PlayerManager;
use crate::testing_atk::build_atk_damage_indiv;

#[cfg(not(tarpaulin_include))]
pub fn testing_character() -> Character {
    let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
    let root_path = "./tests/offlines";
    let c = Character::try_new_from_json(file_path, root_path, false, &testing_equipment());
    let mut c = c.unwrap();
    let atk = build_atk_damage_indiv();
    c.attacks_list.insert(atk.name.clone(), atk);

    c
}

pub fn testing_equipment() -> HashMap<EquipmentJsonKey, Vec<Equipment>> {
    PlayerManager::testing_pm().equipment_table.clone()
}
