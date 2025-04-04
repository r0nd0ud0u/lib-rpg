#![allow(dead_code)]

use crate::{
    attack_type::AttackType,
    common::{all_target_const::*, reach_const::*},
    testing_effect::build_dmg_effect_individual,
};

pub fn build_atk_damage1() -> AttackType {
    AttackType {
        name: "atk1".to_owned(),
        mana_cost: 10,
        target: TARGET_ENNEMY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        all_effects: vec![build_dmg_effect_individual()],
        ..Default::default()
    }
}
