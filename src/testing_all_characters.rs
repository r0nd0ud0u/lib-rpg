#![allow(dead_code)]

use crate::character::Character;
use crate::testing_atk::build_atk_damage1;

pub fn testing_character() -> Character {
    let file_path = "./tests/characters/test.json"; // Path to the JSON file
    let c = Character::try_new_from_json(file_path);
    let mut c = c.unwrap();
    let atk = build_atk_damage1();
    c.attacks_list.insert(atk.name.clone(), atk);

    c
}
