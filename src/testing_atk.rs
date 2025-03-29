use crate::{
    attack_type::AttackType,
    common::{all_target_const::*, reach_const::*},
};

pub fn build_atk_damage1() -> AttackType {
    AttackType {
        name: "atk1".to_owned(),
        mana_cost: 10,
        target: TARGET_ENNEMY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        ..Default::default()
    }
}
