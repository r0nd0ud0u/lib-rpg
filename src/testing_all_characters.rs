#![allow(dead_code)]

use std::collections::HashMap;

use crate::character::Character;
use crate::data_manager::DataManager;
use crate::equipment::{Equipment, EquipmentJsonKey};
use crate::game_manager::GameManager;
use crate::players_manager::PlayerManager;
use crate::testing_atk::build_atk_damage_indiv;

pub fn testing_dm() -> DataManager {
    DataManager::try_new("./tests/offlines").unwrap()
}

pub fn testing_all_equipment() -> HashMap<EquipmentJsonKey, Vec<Equipment>> {
    testing_dm().equipment_table.clone()
}

pub fn testing_pm() -> PlayerManager {
    let dm = testing_dm();
    let mut pl = PlayerManager::new(dm.equipment_table);
    pl.active_heroes = dm.all_heroes.clone();
    // All the bosses are active
    pl.active_bosses = dm.all_bosses.clone();
    pl.current_player = pl.active_heroes[0].clone();
    pl
}

pub fn testing_game_manager() -> crate::game_manager::GameManager {
    let dm = testing_dm();
    // init gm
    let mut gm = GameManager::new("./tests/offlines", dm.equipment_table.clone());
    // All the bosses are active
    gm.pm = testing_pm();
    gm
}

#[cfg(not(tarpaulin_include))]
pub fn testing_character() -> Character {
    let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
    let root_path = "./tests/offlines";
    let c = Character::try_new_from_json(file_path, root_path, false, &testing_all_equipment());
    let mut c = c.unwrap();
    let atk = build_atk_damage_indiv();
    c.attacks_list.insert(atk.name.clone(), atk);

    c
}

pub fn dxrpg_dm() -> DataManager {
    DataManager::try_new("./offlines").unwrap()
}

pub fn dxrpg_all_equipment() -> HashMap<EquipmentJsonKey, Vec<Equipment>> {
    dxrpg_dm().equipment_table.clone()
}

pub fn dxrpg_pm() -> PlayerManager {
    let dm = dxrpg_dm();
    let mut pl = PlayerManager::new(dm.equipment_table);
    pl.active_heroes = dm.all_heroes.clone();
    // All the bosses are active
    pl.active_bosses = dm.all_bosses.clone();
    pl.current_player = pl.active_heroes[0].clone();
    pl
}

pub fn dxrpg_game_manager() -> crate::game_manager::GameManager {
    let dm = dxrpg_dm();
    // init gm
    let mut gm = GameManager::new("./offlines", dm.equipment_table.clone());
    // All the bosses are active
    gm.pm = dxrpg_pm();
    gm
}
