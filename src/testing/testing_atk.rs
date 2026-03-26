#![allow(dead_code)]

#[cfg(not(tarpaulin_include))]
use crate::{
    character_mod::attack_type::AttackType,
    common::constants::{all_target_const::*, reach_const::*},
    testing::testing_effect::{build_dmg_effect_individual, build_hot_effect_individual},
};

#[cfg(not(tarpaulin_include))]
pub fn build_atk_damage_indiv() -> AttackType {
    AttackType {
        name: "atk1".to_owned(),
        vigor_cost: 10,
        mana_cost: 10,
        berseck_cost: 10,
        target: TARGET_ENNEMY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        all_effects: vec![build_dmg_effect_individual().input_effect_param],
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg(not(test))]
pub fn build_atk_damage_zone() -> AttackType {
    use crate::testing::testing_effect::build_dmg_effect_zone;

    AttackType {
        name: "atk1_zone".to_owned(),
        mana_cost: 10,
        target: TARGET_ENNEMY.to_owned(),
        reach: ZONE.to_owned(),
        all_effects: vec![build_dmg_effect_zone().input_effect_param],
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg(not(test))]
pub fn build_atk_berseck_damage1() -> AttackType {
    AttackType {
        name: "atk1".to_owned(),
        berseck_cost: 2,
        target: TARGET_ENNEMY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        all_effects: vec![build_dmg_effect_individual().input_effect_param],
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_atk_heal1_indiv() -> AttackType {
    AttackType {
        name: "atk_heal1_indiv".to_owned(),
        berseck_cost: 2,
        target: TARGET_ALLY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        all_effects: vec![build_hot_effect_individual().input_effect_param],
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_atk_heal1_zone() -> AttackType {
    use crate::testing::testing_effect::build_hot_effect_zone;

    AttackType {
        name: "atk1".to_owned(),
        mana_cost: 10,
        target: TARGET_ALLY.to_owned(),
        reach: ZONE.to_owned(),
        all_effects: vec![build_hot_effect_zone().input_effect_param],
        ..Default::default()
    }
}
