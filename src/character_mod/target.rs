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
mod tests {
    use crate::character_mod::effect::EffectParam;

    use super::*;
    use crate::common::constants::reach_const::ZONE;

    fn make_target(
        target_kind: &str,
        reach: &str,
        launcher_id: &str,
        target_id: &str,
        launcher_kind: CharacterKind,
        target_kind_chara: CharacterKind,
    ) -> TargetData {
        TargetData {
            launcher_id_name: launcher_id.to_string(),
            target_id_name: target_id.to_string(),
            launcher_chara_kind: launcher_kind,
            target_chara_kind: target_kind_chara,
            effect_param: EffectParam {
                target_kind: target_kind.to_string(),
                reach: reach.to_string(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn unit_is_potential_target_himself_wrong_id() {
        let td = make_target(
            TARGET_HIMSELF,
            "",
            "hero1",
            "hero2",
            CharacterKind::Hero,
            CharacterKind::Hero,
        );
        assert!(!td.is_potential_target_on_effect());
    }

    #[test]
    fn unit_is_potential_target_himself_same_id() {
        let td = make_target(
            TARGET_HIMSELF,
            "",
            "hero1",
            "hero1",
            CharacterKind::Hero,
            CharacterKind::Hero,
        );
        assert!(td.is_potential_target_on_effect());
    }

    #[test]
    fn unit_is_potential_target_only_ally_same_id() {
        let td = make_target(
            TARGET_ONLY_ALLY,
            "",
            "hero1",
            "hero1",
            CharacterKind::Hero,
            CharacterKind::Hero,
        );
        assert!(!td.is_potential_target_on_effect());
    }

    #[test]
    fn unit_is_potential_target_only_ally_diff_id() {
        let td = make_target(
            TARGET_ONLY_ALLY,
            "",
            "hero1",
            "hero2",
            CharacterKind::Hero,
            CharacterKind::Hero,
        );
        assert!(td.is_potential_target_on_effect());
    }

    #[test]
    fn unit_is_potential_target_ally_effect_on_enemy() {
        let td = make_target(
            TARGET_ALLY,
            "",
            "hero1",
            "boss1",
            CharacterKind::Hero,
            CharacterKind::Boss,
        );
        assert!(!td.is_potential_target_on_effect());
    }

    #[test]
    fn unit_is_potential_target_enemy_effect_on_ally() {
        let td = make_target(
            TARGET_ENNEMY,
            "",
            "hero1",
            "hero2",
            CharacterKind::Hero,
            CharacterKind::Hero,
        );
        assert!(!td.is_potential_target_on_effect());
    }

    #[test]
    fn unit_is_potential_target_ally_zone_self() {
        let td = make_target(
            TARGET_ALLY,
            ZONE,
            "hero1",
            "hero1",
            CharacterKind::Hero,
            CharacterKind::Hero,
        );
        assert!(!td.is_potential_target_on_effect());
    }

    #[test]
    fn unit_is_target_ally() {
        assert!(is_target_ally(TARGET_ALLY));
        assert!(is_target_ally(TARGET_ALL_ALLIES));
        assert!(is_target_ally(TARGET_HIMSELF));
        assert!(is_target_ally(TARGET_ONLY_ALLY));
        assert!(!is_target_ally(TARGET_ENNEMY));
    }
}
