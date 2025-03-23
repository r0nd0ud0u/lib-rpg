use serde::{Deserialize, Serialize};

use crate::effect::EffectParam;

/// Defines the parameters of an attack.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AttackType {
    /// Name of the attack
    pub name: String,
    pub level: u8,
    pub mana_cost: u32,
    pub vigor_cost: u32,
    pub berseck_cost: u32,
    pub target: String,
    pub reach: String,
    pub name_photo: String,
    pub all_effects: Vec<EffectParam>,
    pub form: String,
}
