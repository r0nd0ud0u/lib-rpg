use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::common::{effect_const::*, stats_const::*};

/// Define the parameters of an effect.
/// An effect can be enabled from an attack, a passive power or an object.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectParam {
    /// Received
    /// Name of the effect
    pub effect_type: String,
    /// Duration of the effect
    pub nb_turns: i64,
    /// TODO sub_value_effect
    pub sub_value_effect: i64,
    /// TODO target of the effect, ally or ennemy
    pub target: String,
    /// TODO, reach of the effect, zone or individual
    pub reach: String,
    /// Name of the targeted stat
    pub stats_name: String,
    /// Value of the effect
    pub value: i64,

    /// Processed
    /// TODO
    pub updated: bool,
    /// TODO from a magical attack ?or is magical effect ?
    pub is_magic_atk: bool,
    /// Lasting turns
    pub counter_turn: i64,
    /// Number of applies
    pub number_of_applies: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectOutcome {
    pub log_display: String,
    pub new_effects: Vec<EffectParam>,
    pub full_atk_amount_tx: i64,
    pub real_amount_tx: i64,
    pub target_name: String,
    pub atk: String,
    pub is_critical: bool,
    /// Updated effect param after apply on the target
    pub new_effect_param: EffectParam,
}

pub fn is_effet_hot_or_dot(effect_name: &str) -> bool {
    let effects_hot_or_dot: HashSet<&str> = [
        EFFECT_VALUE_CHANGE,
        EFFECT_REPEAT_AS_MANY_AS,
        EFFECT_PERCENT_CHANGE,
        EFFECT_NB_DECREASE_ON_TURN,
    ]
    .iter()
    .cloned()
    .collect();
    effects_hot_or_dot.contains(effect_name)
}

pub fn is_active_effect_from_launch(effect_name: &str) -> bool {
    let active_effects_on_launch: HashSet<&str> = [
        EFFECT_NB_DECREASE_BY_TURN,
        EFFECT_NB_COOL_DOWN,
        EFFECT_REINIT,
        EFFECT_DELETE_BAD,
        EFFECT_IMPROVE_HOTS,
        EFFECT_BOOSTED_BY_HOTS,
        EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT,
        EFFECT_IMPROVEMENT_STAT_BY_VALUE,
        EFFECT_IMPROVE_BY_PERCENT_CHANGE,
        EFFECT_INTO_DAMAGE,
        EFFECT_NEXT_HEAL_IS_CRIT,
        EFFECT_BUF_MULTI,
        EFFECT_BLOCK_HEAL_ATK,
        EFFECT_BUF_VALUE_AS_MUCH_AS_HEAL,
    ]
    .iter()
    .cloned()
    .collect();
    active_effects_on_launch.contains(effect_name)
}

pub fn is_boosted_by_crit(effect_name: &str) -> bool {
    let boosted_effects_by_crit: HashSet<&str> = [
        EFFECT_IMPROVE_BY_PERCENT_CHANGE,
        EFFECT_IMPROVEMENT_STAT_BY_VALUE,
        EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT,
        EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT,
        EFFECT_CHANGE_HEAL_RX_BY_PERCENT,
        EFFECT_CHANGE_HEAL_TX_BY_PERCENT,
        EFFECT_INTO_DAMAGE,
    ]
    .iter()
    .cloned()
    .collect();
    boosted_effects_by_crit.contains(effect_name)
}

pub fn is_effect_processed(ep: &EffectParam, from_launch: bool, reload: bool) -> bool {
    if !from_launch && is_active_effect_from_launch(&ep.effect_type) {
        return true;
    }
    if (ep.stats_name == DODGE || ep.stats_name == CRITICAL_STRIKE) && (!from_launch && !reload) {
        return true;
    }
    if ep.stats_name != HP && ep.effect_type == EFFECT_VALUE_CHANGE && (!from_launch && !reload) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_effet_hot_or_dot() {
        assert_eq!(is_effet_hot_or_dot(EFFECT_VALUE_CHANGE), true);
        assert_eq!(is_effet_hot_or_dot("hehe"), false);
    }

    #[test]
    fn test_is_active_effect_from_launch() {
        assert_eq!(
            is_active_effect_from_launch(EFFECT_NB_DECREASE_BY_TURN),
            true
        );
        assert_eq!(is_effet_hot_or_dot("hehe"), false);
    }

    #[test]
    fn test_is_boosted_by_crit() {
        assert_eq!(is_boosted_by_crit(EFFECT_IMPROVE_BY_PERCENT_CHANGE), true);
        assert_eq!(is_effet_hot_or_dot("hehe"), false);
    }
}
