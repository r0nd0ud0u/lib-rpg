use std::collections::HashSet;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::common::{effect_const::*, stats_const::HP};

/// Define the parameters of an effect.
/// An effect can be enabled from an attack, a passive power or an object.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EffectParam {
    /// Received
    /// Name of the effect
    #[serde(rename = "Type")]
    pub effect_type: String,
    /// Duration of the effect
    #[serde(rename = "Tours actifs")]
    pub nb_turns: i64,
    /// TODO sub_value_effect
    #[serde(rename = "Valeur de l'effet")]
    pub sub_value_effect: i64,
    /// TODO target of the effect, ally or ennemy
    #[serde(rename = "Cible")]
    pub target: String,
    /// TODO, reach of the effect, zone or individual
    #[serde(rename = "Portée")]
    pub reach: String,
    /// Name of the targeted stat
    #[serde(rename = "Stat")]
    pub stats_name: String,
    /// Value of the effect
    #[serde(rename = "Value")]
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
    pub full_atk_amount_tx: i64,
    pub real_hp_amount_tx: i64,
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

pub fn is_hot(effect_name: &str, stats: &str, value: i64) -> bool {
    is_effet_hot_or_dot(effect_name) && stats == HP && value > 0
}

pub fn is_boosted_by_crit(effect_name: &str) -> bool {
    let boosted_effects_by_crit: HashSet<&str> = [
        EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE,
        EFFECT_IMPROVE_MAX_STAT_BY_VALUE,
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

pub fn is_effect_only_at_atk_launch(effect_name: &str) -> bool {
    let effects: HashSet<&str> = [
        EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE,
        EFFECT_IMPROVE_MAX_STAT_BY_VALUE,
        EFFECT_BUF_VALUE_AS_MUCH_AS_HEAL,
    ]
    .iter()
    .cloned()
    .collect();
    effects.contains(effect_name)
}

pub fn process_decrease_on_turn(ep: &EffectParam) -> i64 {
    let mut nb_of_applies = 0;
    let mut counter = ep.sub_value_effect;
    let step_limit = (100 / counter) + 1; // Calculate once

    let mut rng = rand::rng();

    while counter > 0 {
        let max_limit = step_limit * counter;
        if rng.random_range(0..=100) <= max_limit {
            nb_of_applies += 1;
        } else {
            break;
        }
        counter -= 1;
    }
    nb_of_applies
}

#[cfg(test)]
mod tests {
    use crate::{common::all_target_const::TARGET_ALLY, target::is_target_ally};

    use super::*;

    #[test]
    fn tunit_is_effet_hot_or_dot() {
        assert!(is_effet_hot_or_dot(EFFECT_VALUE_CHANGE));
        assert!(!is_effet_hot_or_dot("hehe"));
    }

    #[test]
    fn unit_is_boosted_by_crit() {
        assert!(is_boosted_by_crit(EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE));
        assert!(!is_effet_hot_or_dot("hehe"));
    }

    #[test]
    fn unit_process_decrease_on_turn() {
        let ep = EffectParam {
            sub_value_effect: 3,
            ..Default::default()
        };
        let result = process_decrease_on_turn(&ep);
        assert!((0..=3).contains(&result));
    }

    #[test]
    fn unit_is_effect_only_at_atk_launch() {
        assert!(is_effect_only_at_atk_launch(
            EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE
        ));
        assert!(!is_effect_only_at_atk_launch("hehe"));
    }

    #[test]
    fn unit_is_target_ally() {
        assert!(is_target_ally(TARGET_ALLY));
        assert!(!is_target_ally("hehe"));
    }

    #[test]
    fn unit_is_hot() {
        let result = is_hot(EFFECT_BLOCK_HEAL_ATK, HP, 0);
        assert!(!result);
        let result = is_hot(EFFECT_VALUE_CHANGE, HP, 0);
        assert!(!result);
        let result = is_hot(EFFECT_VALUE_CHANGE, HP, 10);
        assert!(result);
        let result = is_hot(EFFECT_VALUE_CHANGE, HP, -10);
        assert!(!result);
    }
}
