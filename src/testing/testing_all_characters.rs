#![allow(dead_code)]

use std::collections::HashMap;

#[cfg(not(tarpaulin_include))]
use crate::character_mod::character::Character;
use crate::character_mod::equipment::{Equipment, EquipmentJsonKey};
use crate::common::constants::paths_const::TEST_OFFLINE_ROOT;
use crate::server::data_manager::DataManager;
use crate::server::game_manager::GameManager;
use crate::server::players_manager::PlayerManager;
use crate::testing::testing_atk::build_atk_damage_indiv;
#[cfg(not(tarpaulin_include))]
use crate::testing::testing_atk::build_atk_heal1_indiv;

pub fn testing_dm() -> DataManager {
    DataManager::try_new(*TEST_OFFLINE_ROOT).unwrap()
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
    // find test
    let test_hero = pl
        .active_heroes
        .iter()
        .find(|h| h.id_name == "test_#1")
        .unwrap();
    pl.current_player = test_hero.clone();
    pl
}

pub fn testing_game_manager() -> GameManager {
    let dm = testing_dm();
    // init gm
    let mut gm = GameManager::new(*TEST_OFFLINE_ROOT, dm.equipment_table.clone());
    // All the bosses are active
    gm.pm = testing_pm();
    gm
}

pub fn testing_test_ally1_vs_test_boss1() -> (GameManager, String, String) {
    let mut gm = testing_game_manager();
    gm.start_game();
    let hero_launcher_id_name = "test_#1".to_string();
    while gm.pm.current_player.id_name != hero_launcher_id_name {
        gm.new_round();
    }
    (gm, hero_launcher_id_name, "test_boss1_#1".to_string())
}

#[cfg(not(tarpaulin_include))]
pub fn testing_character() -> Character {
    let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
    let c = Character::try_new_from_json(
        file_path,
        *TEST_OFFLINE_ROOT,
        false,
        &testing_all_equipment(),
    );
    let mut c = c.unwrap();
    let atk = build_atk_damage_indiv();
    c.attacks_list.insert(atk.name.clone(), atk);
    let atk = build_atk_heal1_indiv();
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

pub fn dxrpg_game_manager() -> GameManager {
    let dm: DataManager = dxrpg_dm();
    // init gm
    let mut gm = GameManager::new("./offlines", dm.equipment_table.clone());
    // All the bosses are active
    gm.pm = dxrpg_pm();
    gm
}

#[cfg(test)]
mod tests {
    use crate::{server::data_manager::DataManager, testing::testing_all_characters::dxrpg_dm};

    #[test]
    fn unit_dxrpg_game_manager() {
        let gm = super::dxrpg_game_manager();
        assert_eq!(gm.pm.active_heroes.len(), 4);
        assert_eq!(gm.pm.active_bosses.len(), 2);

        // test all_buffer length
        assert_eq!(
            gm.pm.active_bosses[0]
                .character_rounds_info
                .all_buffers
                .len(),
            1
        );
    }

    #[test]
    fn unit_dxrpg_dm() {
        let dm: DataManager = dxrpg_dm();
        assert_eq!(dm.all_bosses[0].character_rounds_info.all_buffers.len(), 1);
    }
}
