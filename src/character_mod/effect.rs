use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    character_mod::buffers::{BufKinds, Buffer},
    common::{
        constants::{
            all_target_const::TARGET_ALLY,
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
    /// from a magical attack ?or is magical effect ?
    #[serde(rename = "IsMagicEffect")]
    pub is_magic_atk: bool,
    /// Conditions for the effect
    #[serde(rename = "Conditions")]
    pub conditions: Vec<Condition>,
    #[serde(rename = "Buffer")]
    pub buffer: Buffer,
    #[serde(rename = "is_passive")]
    pub is_passive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessedEffectParam {
    pub input_effect_param: EffectParam,
    /// Lasting turns
    pub counter_turn: i64,
    /// Number of applies
    pub number_of_applies: i64,
    pub log: LogData,
    /// MultiValue multiplier to apply after the power-scaled formula in is_receiving_atk.
    /// Defaults to 1 (no multiplication). Set by process_all_effects when a MultiValue
    /// effect precedes a heal effect so the multiplier survives the launcher→target boundary.
    pub heal_multiplier: i64,
}

impl Default for ProcessedEffectParam {
    fn default() -> Self {
        Self {
            input_effect_param: EffectParam::default(),
            counter_turn: 0,
            number_of_applies: 0,
            log: LogData::default(),
            heal_multiplier: 1,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EffectOutcome {
    /// Raw damage before armor mitigation (0 for non-damage effects).
    pub pre_armor_amount_tx: i64,
    /// Damage/heal after armor, buf/debuf, and blocking — before HP cap.
    pub full_amount_tx: i64,
    /// Actual HP change after HP cap (real amount applied to the target).
    pub real_amount_tx: i64,
    pub target_id_name: String,
    pub is_critical: bool,
    pub aggro_generated: u64,
    /// Set to true when RemoveOneDebuf successfully removed a debuff.
    pub debuff_removed: bool,
}

pub fn is_effet_hot_or_dot(buf_types: &BufKinds) -> bool {
    let effects_hot_or_dot: Vec<BufKinds> = [
        BufKinds::ChangeCurrentStatByValue,
        BufKinds::RepeatAsManyAsPossible,
        BufKinds::ChangeCurrentStatByPercentage,
        BufKinds::DecreasingRateOnTurn,
    ]
    .to_vec();
    effects_hot_or_dot.contains(buf_types)
}

pub fn is_hot(buf_types: &BufKinds, stats: &str, value: i64) -> bool {
    is_effet_hot_or_dot(buf_types) && stats == HP && value > 0
}

/// Returns true if the given effect represents a debuff (harmful to the character who has it).
///
/// Used by RemoveOneDebuf to identify which effects should be removed.
pub fn is_debuf_effect(ep: &EffectParam) -> bool {
    match ep.buffer.kind {
        // Blocks healing — always harmful regardless of value
        BufKinds::BlockHealAtk => true,
        // Positive value = takes MORE damage = debuff
        BufKinds::DamageRxPercent => ep.buffer.value > 0,
        // Negative value = deals/receives less healing or damage = debuff
        BufKinds::DamageTxPercent | BufKinds::HealTxPercent | BufKinds::HealRxPercent => {
            ep.buffer.value < 0
        }
        // For all other kinds (DOTs, stat reductions, etc.): negative value = harmful
        _ => ep.buffer.value < 0,
    }
}

pub fn is_boosted_by_crit(buf_types: &BufKinds) -> bool {
    let boosted_effects_by_crit: Vec<BufKinds> = [
        BufKinds::ChangeMaxStatByPercentage,
        BufKinds::ChangeMaxStatByValue,
        BufKinds::DamageRxPercent,
        BufKinds::DamageTxPercent,
        BufKinds::HealRxPercent,
        BufKinds::HealTxPercent,
        BufKinds::PercentageIntoDamages,
    ]
    .to_vec();
    boosted_effects_by_crit.contains(buf_types)
}

pub fn is_effect_only_at_atk_launch(buf_types: &BufKinds) -> bool {
    let effects: Vec<BufKinds> = [
        BufKinds::ChangeMaxStatByPercentage,
        BufKinds::ChangeMaxStatByValue,
        BufKinds::AddAsMuchAsHp,
    ]
    .to_vec();
    effects.contains(buf_types)
}

pub fn process_decrease_on_turn(ep: &EffectParam, counter_turn: i64) -> i64 {
    let total = ep.sub_value_effect;
    if total <= 0 {
        return 0;
    }
    if counter_turn > 0 {
        // Per-tick check: probability decreases as counter_turn increases.
        // counter 1 → 100%, counter 2 → 67%, counter 3 → 33% (for total=3).
        if counter_turn > total {
            return 0;
        }
        let threshold = ((total - counter_turn + 1) as f64 / total as f64 * 100.0).round() as i64;
        let mut rng = rand::rng();
        return if rng.random_range(0..=100) <= threshold {
            1
        } else {
            0
        };
    }
    // Launch: cumulative applies — first roll always succeeds, each subsequent roll less likely.
    let mut nb_of_applies = 0;
    let mut counter = total;
    let mut rng = rand::rng();
    while counter > 0 {
        let threshold = (counter as f64 / total as f64 * 100.0).round() as i64;
        if rng.random_range(0..=100) <= threshold {
            nb_of_applies += 1;
        } else {
            break;
        }
        counter -= 1;
    }
    nb_of_applies
}

pub fn build_energy_effect(stat_name: &str, value: i64) -> EffectParam {
    EffectParam {
        nb_turns: 1,
        target_kind: TARGET_ALLY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        buffer: Buffer {
            kind: BufKinds::ChangeCurrentStatByValue,
            value,
            stats_name: stat_name.to_owned(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn build_resurrect_effect(value: i64) -> EffectParam {
    EffectParam {
        nb_turns: 1,
        target_kind: TARGET_ALLY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        buffer: Buffer {
            kind: BufKinds::Resurrect,
            value,
            stats_name: HP.to_owned(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn build_hp_effect(value: i64, is_zone: bool) -> EffectParam {
    EffectParam {
        nb_turns: 1,
        target_kind: TARGET_ALLY.to_owned(),
        reach: if is_zone {
            ZONE.to_owned()
        } else {
            INDIVIDUAL.to_owned()
        },
        buffer: Buffer {
            kind: BufKinds::ChangeCurrentStatByValue,
            value,
            stats_name: HP.to_owned(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character_mod::target::is_target_ally,
        common::constants::{
            all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
            reach_const::ZONE,
        },
    };

    use super::*;

    #[test]
    fn unit_is_effet_hot_or_dot() {
        assert!(is_effet_hot_or_dot(&BufKinds::ChangeCurrentStatByValue));
        assert!(!is_effet_hot_or_dot(&BufKinds::DefaultBuf));
    }

    #[test]
    fn unit_is_boosted_by_crit() {
        assert!(is_boosted_by_crit(&BufKinds::ChangeMaxStatByPercentage));
        assert!(!is_boosted_by_crit(&BufKinds::DefaultBuf));
    }

    #[test]
    fn unit_process_decrease_on_turn() {
        let ep = EffectParam {
            nb_turns: 3,
            sub_value_effect: 3,
            target_kind: TARGET_ENNEMY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            is_magic_atk: false,
            conditions: vec![],
            buffer: Buffer {
                kind: BufKinds::DecreasingRateOnTurn,
                value: 3,
                stats_name: HP.to_owned(),
                ..Default::default()
            },
            is_passive: false,
        };
        let result = process_decrease_on_turn(&ep, 0);
        assert!((0..=3).contains(&result));

        // total <= 0: always returns 0
        let ep_zero = EffectParam {
            sub_value_effect: 0,
            ..Default::default()
        };
        assert_eq!(process_decrease_on_turn(&ep_zero, 0), 0);
        assert_eq!(process_decrease_on_turn(&ep_zero, 1), 0);

        // counter_turn > total: returns 0
        let ep_small = EffectParam {
            sub_value_effect: 2,
            ..Default::default()
        };
        assert_eq!(process_decrease_on_turn(&ep_small, 3), 0);
    }

    #[test]
    fn unit_build_energy_effect() {
        use crate::common::constants::stats_const::MANA;
        let ep = build_energy_effect(MANA, 30);
        assert_eq!(ep.buffer.value, 30);
        assert_eq!(ep.buffer.stats_name, MANA);
        assert_eq!(ep.nb_turns, 1);
        assert_eq!(ep.target_kind, TARGET_ALLY);
    }

    #[test]
    fn unit_build_resurrect_effect() {
        let ep = build_resurrect_effect(50);
        assert_eq!(ep.buffer.value, 50);
        assert_eq!(ep.buffer.kind, BufKinds::Resurrect);
        assert_eq!(ep.target_kind, TARGET_ALLY);
    }

    #[test]
    fn unit_build_hp_effect() {
        let ep = build_hp_effect(20, false);
        assert_eq!(ep.buffer.value, 20);
        let ep_zone = build_hp_effect(20, true);
        assert_eq!(ep_zone.reach, ZONE);
    }

    #[test]
    fn unit_is_effect_only_at_atk_launch() {
        assert!(is_effect_only_at_atk_launch(
            &BufKinds::ChangeMaxStatByPercentage
        ));
        assert!(!is_effect_only_at_atk_launch(&BufKinds::DefaultBuf));
    }

    #[test]
    fn unit_is_target_ally() {
        assert!(is_target_ally(TARGET_ALLY));
        assert!(!is_target_ally("hehe"));
    }

    #[test]
    fn unit_is_hot() {
        let result = is_hot(&BufKinds::BlockHealAtk, HP, 0);
        assert!(!result);
        let result = is_hot(&BufKinds::ChangeCurrentStatByValue, HP, 0);
        assert!(!result);
        let result = is_hot(&BufKinds::ChangeCurrentStatByValue, HP, 10);
        assert!(result);
        let result = is_hot(&BufKinds::ChangeCurrentStatByValue, HP, -10);
        assert!(!result);
    }

    #[test]
    fn unit_is_debuf_effect() {
        let make = |kind: BufKinds, value: i64| EffectParam {
            buffer: Buffer {
                kind,
                value,
                ..Default::default()
            },
            ..Default::default()
        };

        // DOT on HP — classic debuf
        assert!(is_debuf_effect(&make(
            BufKinds::ChangeCurrentStatByValue,
            -10
        )));
        // Positive value on HP — HOT, not a debuf
        assert!(!is_debuf_effect(&make(
            BufKinds::ChangeCurrentStatByValue,
            10
        )));

        // BlockHealAtk is always a debuf regardless of value
        assert!(is_debuf_effect(&make(BufKinds::BlockHealAtk, 0)));
        assert!(is_debuf_effect(&make(BufKinds::BlockHealAtk, 1)));

        // DamageRxPercent: positive = takes MORE damage = debuf
        assert!(is_debuf_effect(&make(BufKinds::DamageRxPercent, 20)));
        // DamageRxPercent: negative = takes LESS damage = buff
        assert!(!is_debuf_effect(&make(BufKinds::DamageRxPercent, -20)));

        // DamageTxPercent: negative = deals LESS damage = debuf
        assert!(is_debuf_effect(&make(BufKinds::DamageTxPercent, -15)));
        // DamageTxPercent: positive = deals MORE damage = buff
        assert!(!is_debuf_effect(&make(BufKinds::DamageTxPercent, 15)));

        // HealTxPercent: negative = heals LESS = debuf
        assert!(is_debuf_effect(&make(BufKinds::HealTxPercent, -10)));
        assert!(!is_debuf_effect(&make(BufKinds::HealTxPercent, 10)));

        // HealRxPercent: negative = receives LESS healing = debuf
        assert!(is_debuf_effect(&make(BufKinds::HealRxPercent, -10)));
        assert!(!is_debuf_effect(&make(BufKinds::HealRxPercent, 10)));

        // Generic negative = debuf, positive = buff
        assert!(is_debuf_effect(&make(
            BufKinds::ChangeCurrentStatByPercentage,
            -5
        )));
        assert!(!is_debuf_effect(&make(
            BufKinds::ChangeCurrentStatByPercentage,
            5
        )));
        assert!(is_debuf_effect(&make(BufKinds::ChangeMaxStatByValue, -50)));
        assert!(!is_debuf_effect(&make(BufKinds::ChangeMaxStatByValue, 50)));
    }
}
