use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    character_mod::buffers::BufTypes,
    common::{
        constants::{
            all_target_const::TARGET_ENNEMY,
            reach_const::{INDIVIDUAL, ZONE},
            stats_const::HP,
        },
        log_data::LogData,
    },
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Condition {
    pub kind: ConditionKind,
    pub value: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionKind {
    #[default]
    NbEnnemiesDied,
}

/// Define the parameters of an effect.
/// An effect can be enabled from an attack, a passive power or an object.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EffectParam {
    /// Received
    /// Name of the effect
    #[serde(rename = "Type")]
    pub buf_type: BufTypes,
    /// Duration of the effect
    #[serde(rename = "Tours actifs")]
    pub nb_turns: i64,
    /// sub_value_effect
    #[serde(rename = "Valeur de l'effet")]
    pub sub_value_effect: i64,
    /// target of the effect, ally or ennemy
    #[serde(rename = "Cible")]
    pub target_kind: String,
    /// reach of the effect, zone or individual
    #[serde(rename = "Portée")]
    pub reach: String,
    /// Name of the targeted stat
    #[serde(rename = "Stat")]
    pub stats_name: String,
    /// Value of the effect
    #[serde(rename = "Value")]
    pub value: i64,
    /// from a magical attack ?or is magical effect ?
    #[serde(rename = "IsMagicEffect")]
    pub is_magic_atk: bool,
    /// Conditions for the effect
    #[serde(rename = "Conditions")]
    pub conditions: Vec<Condition>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessedEffectParam {
    pub input_effect_param: EffectParam,
    /// Lasting turns
    pub counter_turn: i64,
    /// Number of applies
    pub number_of_applies: i64,
    pub log: LogData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EffectOutcome {
    pub full_amount_tx: i64,
    pub real_amount_tx: i64,
    pub target_id_name: String,
    pub is_critical: bool,
    pub aggro_generated: u64,
}

pub fn is_effet_hot_or_dot(buf_types: &BufTypes) -> bool {
    let effects_hot_or_dot: Vec<BufTypes> = [
        BufTypes::ChangeCurrentStatByValue,
        BufTypes::RepeatAsManyAsPossible,
        BufTypes::UpCurrentStatByPercentage,
        BufTypes::DecreasingRateOnTurn,
    ]
    .to_vec();
    effects_hot_or_dot.contains(buf_types)
}

pub fn is_hot(buf_types: &BufTypes, stats: &str, value: i64) -> bool {
    is_effet_hot_or_dot(buf_types) && stats == HP && value > 0
}

pub fn is_boosted_by_crit(buf_types: &BufTypes) -> bool {
    let boosted_effects_by_crit: Vec<BufTypes> = [
        BufTypes::ChangeMaxStatByPercentage,
        BufTypes::ChangeMaxStatByValue,
        BufTypes::DamageRxPercent,
        BufTypes::DamageTxPercent,
        BufTypes::HealRxPercent,
        BufTypes::HealTxPercent,
        BufTypes::PercentageIntoDamages,
    ]
    .to_vec();
    boosted_effects_by_crit.contains(buf_types)
}

pub fn is_effect_only_at_atk_launch(buf_types: &BufTypes) -> bool {
    let effects: Vec<BufTypes> = [
        BufTypes::ChangeMaxStatByPercentage,
        BufTypes::ChangeMaxStatByValue,
        BufTypes::AddAsMuchAsHp,
    ]
    .to_vec();
    effects.contains(buf_types)
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

pub fn build_hp_effect(value: i64, is_zone: bool) -> EffectParam {
    EffectParam {
        buf_type: BufTypes::ChangeCurrentStatByValue,
        nb_turns: 1,
        target_kind: TARGET_ENNEMY.to_owned(),
        reach: if is_zone {
            ZONE.to_owned()
        } else {
            INDIVIDUAL.to_owned()
        },
        stats_name: HP.to_owned(),
        value,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character_mod::target::is_target_ally, common::constants::all_target_const::TARGET_ALLY,
    };

    use super::*;

    #[test]
    fn unit_is_effet_hot_or_dot() {
        assert!(is_effet_hot_or_dot(&BufTypes::ChangeCurrentStatByValue));
        assert!(!is_effet_hot_or_dot(&BufTypes::DefaultBuf));
    }

    #[test]
    fn unit_is_boosted_by_crit() {
        assert!(is_boosted_by_crit(&BufTypes::ChangeMaxStatByPercentage));
        assert!(!is_boosted_by_crit(&BufTypes::DefaultBuf));
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
            &BufTypes::ChangeMaxStatByPercentage
        ));
        assert!(!is_effect_only_at_atk_launch(&BufTypes::DefaultBuf));
    }

    #[test]
    fn unit_is_target_ally() {
        assert!(is_target_ally(TARGET_ALLY));
        assert!(!is_target_ally("hehe"));
    }

    #[test]
    fn unit_is_hot() {
        let result = is_hot(&BufTypes::BlockHealAtk, HP, 0);
        assert!(!result);
        let result = is_hot(&BufTypes::ChangeCurrentStatByValue, HP, 0);
        assert!(!result);
        let result = is_hot(&BufTypes::ChangeCurrentStatByValue, HP, 10);
        assert!(result);
        let result = is_hot(&BufTypes::ChangeCurrentStatByValue, HP, -10);
        assert!(!result);
    }
}
