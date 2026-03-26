use serde::{Deserialize, Serialize};

use crate::character_mod::effect::EffectParam;

/// Define all the parameters of a Power.
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub enum PowerKind {
    /// Enables the critical of the next heal atk after a critical on damage atk
    #[default]
    IsCritHealAfterCrit,
    /// Enables the power to heal the most needy ally using damage tx of previous turn
    IsDamageTxHealNeedyAlly,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct Power {
    pub kind: PowerKind,
    pub all_effects: Vec<EffectParam>,
    pub is_passive: bool,
}
