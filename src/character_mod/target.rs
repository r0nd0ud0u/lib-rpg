use indexmap::IndexSet;

use crate::{
    character_mod::character::CharacterKind,
    character_mod::effect::EffectParam,
    common::constants::{all_target_const::*, reach_const::ZONE},
};

/// Define all the parameters of target info during a round
#[derive(Default, Debug, Clone)]
pub struct TargetData {
    pub launcher_id_name: String,
    pub target_id_name: String,
    pub target_chara_kind: CharacterKind,
    pub launcher_chara_kind: CharacterKind,
    pub effect_param: EffectParam,
}

impl TargetData {
    pub fn is_potential_target_on_effect(&self) -> bool {
        let is_ally = self.launcher_chara_kind == self.target_chara_kind;
        if self.effect_param.target_kind == TARGET_HIMSELF
            && self.launcher_id_name != self.target_id_name
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is himself.",
                self.effect_param.buffer.kind,
                self.target_id_name
            );
            return false;
        }
        if self.effect_param.target_kind == TARGET_ONLY_ALLY
            && self.launcher_id_name == self.target_id_name
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is only ally but launcher is himself.",
                self.effect_param.buffer.kind,
                self.target_id_name
            );
            return false;
        }
        if !is_ally && is_target_ally(&self.effect_param.target_kind) {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is ally but launcher is ennemy.",
                self.effect_param.buffer.kind,
                self.target_id_name
            );
            return false;
        }
        if is_ally && self.effect_param.target_kind == TARGET_ENNEMY {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is ennemy but launcher is ally.",
                self.effect_param.buffer.kind,
                self.target_id_name
            );
            return false;
        }
        if self.effect_param.target_kind == TARGET_ALLY
            && self.effect_param.reach == ZONE
            && self.launcher_id_name == self.target_id_name
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is ally but launcher is himself.",
                self.effect_param.buffer.kind,
                self.target_id_name
            );
            return false;
        }
        true
    }
}

pub fn is_target_ally(target: &str) -> bool {
    let targets: IndexSet<&str> = [
        TARGET_ALLY,
        TARGET_ALL_ALLIES,
        TARGET_HIMSELF,
        TARGET_ONLY_ALLY,
    ]
    .iter()
    .cloned()
    .collect();
    targets.contains(target)
}

#[cfg(test)]
mod tests {}
