use std::vec;

use serde::{Deserialize, Serialize};

use crate::{
    common::{all_target_const::TARGET_ALLY, reach_const::INDIVIDUAL},
    effect::EffectParam,
};

/// Defines the parameters of an attack.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AttackType {
    /// Name of the attack
    pub name: String,
    pub level: u8,
    pub mana_cost: u64,
    pub vigor_cost: u64,
    pub berseck_cost: u64,
    pub target: String,
    pub reach: String,
    pub name_photo: String,
    pub all_effects: Vec<EffectParam>,
    pub form: String,
}

impl Default for AttackType {
    fn default() -> Self {
        AttackType {
            name: "".to_owned(),
            level: 0,
            mana_cost: 0,
            vigor_cost: 0,
            berseck_cost: 0,
            target: TARGET_ALLY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            name_photo: "".to_owned(),
            all_effects: vec![],
            form: "".to_owned(),
        }
    }
}
