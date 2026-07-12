use serde::{Deserialize, Serialize};

use crate::{
    character_mod::{
        attack_type::AttackType,
        buffers::BufKinds,
        effect::{EffectOutcome, ProcessedEffectParam},
    },
    common::constants::stats_const::HP,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameAtkEffect {
    pub processed_effect_param: ProcessedEffectParam,
    pub atk_type: AttackType,
    pub launching_turn: usize,
    pub launching_round: usize,
    pub effect_outcome: EffectOutcome,
}

impl GameAtkEffect {
    /// Returns the text line for the attack log, or `None` if this effect should be hidden.
    ///
    /// This is the single source of truth for per-effect log text used in both the gameboard
    /// and the log sheet (`build_logs_atk`). Format uses `←` (target received).
    pub fn log_text(&self) -> Option<String> {
        let kind = &self.processed_effect_param.input_effect_param.buffer.kind;
        let target = &self.effect_outcome.target_id_name;
        let real = self.effect_outcome.real_amount_tx;
        let full = self.effect_outcome.full_amount_tx;
        let pre = self.effect_outcome.pre_armor_amount_tx;
        let stat = &self
            .processed_effect_param
            .input_effect_param
            .buffer
            .stats_name;
        let number_of_applies = self.processed_effect_param.number_of_applies;
        let buf_value = self.processed_effect_param.input_effect_param.buffer.value;

        match kind {
            BufKinds::CooldownTurnsNumber => {
                Some(format!("{target} ← Cooldown for {buf_value} turns"))
            }
            BufKinds::ConditionDamagePrevTurn => {
                if number_of_applies > 0 {
                    Some(format!("{target} ← ✓ Condition: damage last turn"))
                } else {
                    Some(format!(
                        "{target} ← ✗ Condition: damage last turn (×multiplier skipped)"
                    ))
                }
            }
            BufKinds::MultiValue => Some(format!("{target} ← Heal ×{buf_value}")),
            BufKinds::RemoveOneDebuf => {
                if self.effect_outcome.debuff_removed {
                    Some(format!("{target} ← debuff removed"))
                } else {
                    None
                }
            }
            BufKinds::ReinitBuf => {
                if stat.is_empty() {
                    None
                } else {
                    Some(format!("{target} ← {stat} effects reset"))
                }
            }
            BufKinds::BoostHotsByPercentage => {
                if full > 0 {
                    Some(format!("{target} ← HOTs +{buf_value}% (+{full} HP/turn)"))
                } else {
                    Some(format!("{target} ← HOTs +{buf_value}%"))
                }
            }
            BufKinds::BoostBufByHotsNumberInPercentage => Some(format!(
                "{target} ← +{buf_value}% heal boost per active HOT"
            )),
            _ => {
                let is_hp = stat == HP && *kind != BufKinds::ChangeMaxStat;
                let is_damage = is_hp && (real < 0 || full < 0);
                if is_hp {
                    if is_damage {
                        // `pre`  = raw pre-armor/block/crit damage
                        // `full` = post-armor/crit/block, before HP cap
                        // `real` = actual HP change after HP cap
                        let mitigated = pre != full; // armor/block/crit changed the amount
                        let hp_capped = full != real; // HP cap limited the amount applied
                        match (mitigated, hp_capped) {
                            (false, false) => Some(format!("{target} ← {real} HP")),
                            (true, false) => Some(format!("{target} ← {real} HP (raw: {pre})")),
                            (false, true) => Some(format!("{target} ← {real} HP (full: {full})")),
                            (true, true) => {
                                Some(format!("{target} ← {real} HP (raw: {pre}, full: {full})"))
                            }
                        }
                    } else if full == real {
                        Some(format!("{target} ← {real} HP ({kind})"))
                    } else {
                        Some(format!("{target} ← {real} HP (full: {full})"))
                    }
                } else if *kind == BufKinds::ChangeMaxStat
                    && self
                        .processed_effect_param
                        .input_effect_param
                        .buffer
                        .is_percent
                {
                    Some(format!("{target} ← {stat} max +{full}%"))
                } else if stat.is_empty() {
                    Some(format!("{target} ← {full} ({kind})"))
                } else {
                    Some(format!("{target} ← {full} {stat} ({kind})"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GameAtkEffect;

    // ── log_text() unit tests ────────────────────────────────────────────────

    fn make_gae_hp(
        real: i64,
        full: i64,
        pre: i64,
        kind: crate::character_mod::buffers::BufKinds,
    ) -> GameAtkEffect {
        use crate::character_mod::{
            buffers::Buffer,
            effect::{EffectOutcome, EffectParam, ProcessedEffectParam},
        };
        use crate::common::constants::stats_const::HP;
        GameAtkEffect {
            processed_effect_param: ProcessedEffectParam {
                input_effect_param: EffectParam {
                    buffer: Buffer {
                        kind: kind.clone(),
                        stats_name: HP.to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            effect_outcome: EffectOutcome {
                real_amount_tx: real,
                full_amount_tx: full,
                pre_armor_amount_tx: pre,
                target_id_name: "Target".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn unit_log_text_hp_damage_no_mitigation() {
        use crate::character_mod::buffers::BufKinds;
        let gae = make_gae_hp(-30, -30, -30, BufKinds::ChangeCurrentStat);
        assert_eq!(
            gae.log_text(),
            Some("Target ← -30 HP".to_string()),
            "no mitigation: real == pre → simple format"
        );
    }

    #[test]
    fn unit_log_text_hp_damage_with_armor() {
        use crate::character_mod::buffers::BufKinds;
        // pre == full (-30): no armor/block mitigation; HP cap limits real to -20.
        let gae = make_gae_hp(-20, -30, -30, BufKinds::ChangeCurrentStat);
        assert_eq!(
            gae.log_text(),
            Some("Target ← -20 HP (full: -30)".to_string()),
            "HP cap only (pre==full): shows (full: N)"
        );
    }

    #[test]
    fn unit_log_text_hp_damage_block_shows_raw() {
        use crate::character_mod::buffers::BufKinds;
        // pre (raw, pre-armor) is -99; block/armor reduces it to full=-8 == real (no HP cap).
        // Shows (raw: -99) so the player can see how effective the block was.
        let gae = make_gae_hp(-8, -8, -99, BufKinds::ChangeCurrentStat);
        assert_eq!(
            gae.log_text(),
            Some("Target ← -8 HP (raw: -99)".to_string()),
            "mitigation only (full==real): shows (raw: pre)"
        );
    }

    #[test]
    fn unit_log_text_hp_damage_armor_and_hp_cap() {
        use crate::character_mod::buffers::BufKinds;
        // pre=-500 (raw), full=-61 (post-armor/crit/block), real=-40 (HP-clamped near death).
        let gae = make_gae_hp(-40, -61, -500, BufKinds::ChangeCurrentStat);
        assert_eq!(
            gae.log_text(),
            Some("Target ← -40 HP (raw: -500, full: -61)".to_string()),
            "mitigation + HP cap: shows (raw: pre, full: full)"
        );
    }

    #[test]
    fn unit_log_text_hp_heal_simple() {
        use crate::character_mod::buffers::BufKinds;
        let gae = make_gae_hp(50, 50, 50, BufKinds::ChangeCurrentStat);
        assert_eq!(
            gae.log_text(),
            Some(format!("Target ← 50 HP ({})", BufKinds::ChangeCurrentStat)),
            "heal with full==real → shows kind in parens"
        );
    }

    #[test]
    fn unit_log_text_hp_heal_capped() {
        use crate::character_mod::buffers::BufKinds;
        let gae = make_gae_hp(30, 50, 50, BufKinds::ChangeCurrentStat);
        assert_eq!(
            gae.log_text(),
            Some("Target ← 30 HP (full: 50)".to_string()),
            "heal capped at HP max → (full: N) format"
        );
    }

    #[test]
    fn unit_log_text_cooldown_uses_buf_value() {
        use crate::character_mod::{
            buffers::{BufKinds, Buffer},
            effect::{EffectOutcome, EffectParam, ProcessedEffectParam},
        };
        let gae = GameAtkEffect {
            processed_effect_param: ProcessedEffectParam {
                input_effect_param: EffectParam {
                    buffer: Buffer {
                        kind: BufKinds::CooldownTurnsNumber,
                        value: 3,
                        ..Default::default()
                    },
                    nb_turns: 5,
                    ..Default::default()
                },
                ..Default::default()
            },
            effect_outcome: EffectOutcome {
                target_id_name: "Hero".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            gae.log_text(),
            Some("Hero ← Cooldown for 3 turns".to_string()),
            "cooldown uses buf_value (not nb_turns)"
        );
    }

    #[test]
    fn unit_log_text_remove_debuf_success() {
        use crate::character_mod::{
            buffers::{BufKinds, Buffer},
            effect::{EffectOutcome, EffectParam, ProcessedEffectParam},
        };
        let gae = GameAtkEffect {
            processed_effect_param: ProcessedEffectParam {
                input_effect_param: EffectParam {
                    buffer: Buffer {
                        kind: BufKinds::RemoveOneDebuf,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            effect_outcome: EffectOutcome {
                target_id_name: "Ally".to_string(),
                debuff_removed: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(gae.log_text(), Some("Ally ← debuff removed".to_string()));
    }

    #[test]
    fn unit_log_text_remove_debuf_noop_is_hidden() {
        use crate::character_mod::{
            buffers::{BufKinds, Buffer},
            effect::{EffectOutcome, EffectParam, ProcessedEffectParam},
        };
        let gae = GameAtkEffect {
            processed_effect_param: ProcessedEffectParam {
                input_effect_param: EffectParam {
                    buffer: Buffer {
                        kind: BufKinds::RemoveOneDebuf,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            effect_outcome: EffectOutcome {
                target_id_name: "Ally".to_string(),
                debuff_removed: false,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(gae.log_text(), None, "no debuff removed → hidden from log");
    }
}
