use anyhow::Result;

use crate::{
    character_mod::{
        attack_type::AttackType,
        buffers::{BufKinds, Buffer, update_damage_by_buf, update_heal_by_multi},
        class::Class,
        effect::{
            self, ConditionKind, EffectParam, ProcessedEffectParam, is_boosted_by_crit,
            is_effet_hot_or_dot, process_decrease_on_turn,
        },
        target::{TargetData, is_target_ally},
    },
    common::{
        constants::{
            all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
            attak_const::{COEFF_CRIT_DMG, COEFF_CRIT_STATS},
            character_const::ULTIMATE_LEVEL,
            reach_const::INDIVIDUAL,
            stats_const::HP,
        },
        log_data::{
            LogData,
            const_colors::{DARK_RED, LIGHT_GREEN},
        },
    },
    server::{
        game_state::GameState,
        players_manager::{DodgeInfo, GameAtkEffect},
    },
    utils::{get_random_nb, softcap_percent},
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Eq, Hash, PartialEq)]
pub enum AmountType {
    DamageRx = 0,
    DamageTx,
    HealRx,
    HealTx,
    OverHealRx,
    Aggro,
    CriticalStrike,
    EnumSize,
}

/// CharacterRoundsInfo
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct CharacterRoundsInfo {
    /// Fight information: Is the random character targeted by the current attack of other character
    #[serde(default, rename = "is_random_target")]
    pub is_random_target: bool,
    /// Fight information: TODO is_heal_atk_blocked
    #[serde(default, rename = "is_heal_atk_blocked")]
    pub is_heal_atk_blocked: bool,
    /// Fight information: Playing the first round of that tour
    #[serde(default, rename = "is_first_round")]
    pub is_first_round: bool,
    #[serde(default, rename = "launchable_atks")]
    pub launchable_atks: Vec<AttackType>,
    /// Fight information: dodge information on atk
    #[serde(default, rename = "dodge-info")]
    pub dodge_info: DodgeInfo,
    /// Fight information: nb-actions-in-round
    #[serde(default, rename = "nb-actions-in-round")]
    pub actions_done_in_round: u64,
    /// Fight information: is_current_target
    #[serde(default, rename = "is-current-target")]
    pub is_current_target: bool,
    /// Fight information: damages transmitted or received through the fight
    #[serde(default, rename = "Tx-rx")]
    pub tx_rx: Vec<HashMap<u64, i64>>,
    /// Fight information: Enabled buf/debuf acquired through the fight
    #[serde(default, rename = "Buf-debuf")]
    pub all_buffers: Vec<Buffer>,
    #[serde(default, rename = "ExpToNextLevel")]
    /// Experience to acquire to upgrade to next level
    pub exp_to_next_level: u64,
    /// Experience of the character, start 0
    #[serde(default, rename = "Experience")]
    pub exp: u64,
    /// Potential target by an individual effect of an atk
    #[serde(default, rename = "is-potential-target")]
    pub is_potential_target: bool,
    #[serde(default, rename = "Effects")]
    pub all_effects: Vec<GameAtkEffect>,
    /// Queue of attack indexes from the scenario pattern, filled on first use and cycled
    #[serde(default, skip)]
    pub atk_pattern_queue: VecDeque<u64>,
    /// Streak-breaker: number of consecutive turns without a critical strike.
    /// Reset to 0 each time a crit lands. Compared against the active threshold
    /// (from rank/class/level or `StreakBreakerCrit` buffer) to guarantee the next crit.
    #[serde(default, skip)]
    pub crit_drought_counter: u32,
    /// Streak-breaker: number of consecutive turns without a successful dodge.
    /// Reset to 0 each time a dodge/block succeeds. Compared against the active
    /// threshold (from rank/class/level or `StreakBreakerDodge` buffer).
    #[serde(default, skip)]
    pub dodge_drought_counter: u32,
}

impl Default for CharacterRoundsInfo {
    fn default() -> Self {
        CharacterRoundsInfo {
            is_random_target: false,
            is_heal_atk_blocked: false,
            is_first_round: true,
            launchable_atks: Vec::new(),
            dodge_info: DodgeInfo::default(),
            actions_done_in_round: 0,
            is_current_target: false,
            tx_rx: vec![HashMap::new()],
            all_buffers: vec![],
            exp_to_next_level: 100,
            exp: 0,
            is_potential_target: false,
            all_effects: vec![],
            atk_pattern_queue: VecDeque::new(),
            crit_drought_counter: 0,
            dodge_drought_counter: 0,
        }
    }
}

/// CharacterRoundsInfo
#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct HotsBufs {
    pub hot_nb: u64,
    pub dot_nb: u64,
    pub buf_nb: u64,
    pub debuf_nb: u64,
    pub hot_txt: Vec<String>,
    pub dot_txt: Vec<String>,
    pub buf_txt: Vec<String>,
    pub debuf_txt: Vec<String>,
}
impl CharacterRoundsInfo {
    pub fn apply_launchable_atks(&mut self, launchable_atks: Vec<AttackType>) {
        self.launchable_atks = launchable_atks;
    }

    /// Add exp and return true if the character can level up
    pub fn add_exp(&mut self, exp_gained: u64) -> bool {
        self.exp += exp_gained;
        if self.exp >= self.exp_to_next_level {
            self.exp -= self.exp_to_next_level;
            return true;
        }
        false
    }

    /// Init all buffers with default values
    pub fn new_buffers(&mut self) {
        for _ in 0..BufKinds::EnumSize as usize {
            self.all_buffers.push(Buffer::default());
        }
    }

    /// Output: hot, dot, buf, debuf
    pub fn get_hot_and_buf_nbs_txts(all_effects: &Vec<GameAtkEffect>) -> HotsBufs {
        let mut hots_bufs = HotsBufs::default();
        for e in all_effects {
            if e.processed_effect_param.input_effect_param.nb_turns < 2 {
                continue;
            }
            let txt = Self::get_hot_and_buf_texts(e);
            if effect::is_hot(
                &e.processed_effect_param.input_effect_param.buffer.kind,
                &e.processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name,
                e.processed_effect_param.input_effect_param.buffer.value,
            ) {
                hots_bufs.hot_nb += 1;
                hots_bufs.hot_txt.push(txt);
            } else if e
                .processed_effect_param
                .input_effect_param
                .buffer
                .stats_name
                == HP
                && e.processed_effect_param.input_effect_param.buffer.value < 0
            {
                hots_bufs.dot_nb += 1;
                hots_bufs.dot_txt.push(txt);
            } else if e
                .processed_effect_param
                .input_effect_param
                .buffer
                .stats_name
                == HP
                && e.processed_effect_param.input_effect_param.buffer.value > 0
            {
                hots_bufs.hot_nb += 1;
                hots_bufs.hot_txt.push(txt);
            } else if e.processed_effect_param.input_effect_param.buffer.value > 0 {
                hots_bufs.buf_nb += 1;
                hots_bufs.buf_txt.push(txt);
            } else {
                hots_bufs.debuf_nb += 1;
                hots_bufs.debuf_txt.push(txt);
            }
        }
        hots_bufs
    }

    fn get_hot_and_buf_texts(gae: &GameAtkEffect) -> String {
        let ep = &gae.processed_effect_param.input_effect_param;
        let nb_turns = ep.nb_turns;
        let atk_name = &gae.atk_type.name;

        if ep.buffer.kind == BufKinds::CooldownTurnsNumber {
            return format!("{}: cooldown ({} turns)", atk_name, nb_turns);
        }

        let is_max_stat = ep.buffer.kind == BufKinds::ChangeMaxStatByPercentage
            || ep.buffer.kind == BufKinds::ChangeMaxStatByValue;
        let is_percent = ep.buffer.kind == BufKinds::ChangeMaxStatByPercentage;

        // For current-HP effects show the full computed amount; for max-stat or other stats use raw value.
        let amount = if ep.buffer.stats_name == HP && !is_max_stat {
            gae.effect_outcome.full_amount_tx.abs()
        } else {
            ep.buffer.value.abs()
        };

        let stat_label = if is_max_stat && is_percent {
            format!("{}% max {}", amount, ep.buffer.stats_name)
        } else if is_max_stat {
            format!("{} max {}", amount, ep.buffer.stats_name)
        } else {
            format!("{} {}", amount, ep.buffer.stats_name)
        };

        format!("{}: {} × {} turns", atk_name, stat_label.trim(), nb_turns)
    }

    pub fn is_dodging(&self, target_kind: &str) -> bool {
        self.dodge_info.is_dodging && target_kind == TARGET_ENNEMY
    }

    pub fn is_blocking(&mut self, ep: &EffectParam) -> bool {
        self.dodge_info.is_blocking && ep.buffer.stats_name == HP && ep.target_kind == TARGET_ENNEMY
    }

    pub fn apply_buf_debuf(&self, full_amount: i64, target: &str, is_crit: bool) -> i64 {
        let mut real_amount = full_amount;
        let mut buf_debuf = 0;
        let mut coeff_crit = COEFF_CRIT_DMG;
        // buf debuf heal
        if full_amount > 0 && is_target_ally(target) {
            // Launcher TX: BufTypes::MultiValue
            // To place first

            if let Some(buf_multi) = self.get_buffer_by_type(&BufKinds::MultiValue)
                && buf_multi.value > 0
            {
                real_amount = update_heal_by_multi(real_amount, buf_multi.value);
            }

            self.all_buffers.iter().for_each(|b| {
                match b.kind {
                    // Launcher TX: BufTypes::HealTxPercent, Receiver RX: BufTypes::HealRxPercent, Launcher TX: BufTypes::BoostedByHots
                    BufKinds::HealTxPercent | BufKinds::HealRxPercent | BufKinds::BoostedByHots => {
                        buf_debuf += update_damage_by_buf(b.value, b.is_percent, real_amount);
                    }
                    _ => {}
                }
            });
        }
        // buf debuf damage
        if full_amount < 0 && !is_target_ally(target) {
            // Receiver RX: BufTypes::DamageCritCapped
            if let Some(buf_dmg_crit) = self.get_buffer_by_type(&BufKinds::DamageCritCapped) {
                // improve crit coeff
                coeff_crit += buf_dmg_crit.value as f64;
            }

            self.all_buffers.iter().for_each(|b| {
                match b.kind {
                    // DamageTxPercent, Receiver RX: BufTypes::DamageRxPercent
                    BufKinds::DamageTxPercent | BufKinds::DamageRxPercent => {
                        buf_debuf += update_damage_by_buf(b.value, b.is_percent, real_amount);
                    }
                    _ => {}
                }
            });
        }

        // apply buf/debuf
        real_amount += buf_debuf;
        // is it a critical strike ?
        if is_crit {
            real_amount = (real_amount as f64 * coeff_crit).round() as i64;
        }

        real_amount
    }

    pub fn reset_all_buffers(&mut self) {
        self.all_buffers.clear();
    }

    pub fn increment_counter_effect(&mut self) {
        for gae in self.all_effects.iter_mut() {
            gae.processed_effect_param.counter_turn += 1;
        }
    }

    /// Update all the bufs
    pub fn process_effect_type(
        &mut self,
        ep: &EffectParam,
        atk_name: &str,
    ) -> Result<ProcessedEffectParam> {
        let mut processed_effect_param = ProcessedEffectParam {
            input_effect_param: ep.clone(),
            ..Default::default()
        };
        processed_effect_param.number_of_applies = 1;
        let bug_apply_init = self.get_buffer_by_type(&BufKinds::ApplyEffectInit);
        if let Some(buf) = bug_apply_init
            && buf.value > 0
        {
            processed_effect_param.number_of_applies = buf.value;
        }

        match ep.buffer.kind {
            BufKinds::CooldownTurnsNumber => {
                processed_effect_param.log = LogData {
                    message: format!("Cooldown on {}: {} turns", atk_name, ep.buffer.value),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::DecreasingRateOnTurn => {
                processed_effect_param.number_of_applies = process_decrease_on_turn(ep);
                self.update_buffer(&Buffer {
                    value: processed_effect_param.number_of_applies,
                    is_percent: false,
                    kind: BufKinds::ApplyEffectInit,
                    ..Default::default()
                });
                processed_effect_param.log = LogData {
                    message: format!(
                        "Attack will be applied {} times",
                        processed_effect_param.number_of_applies
                    ),
                    color: "".to_owned(),
                };
            }
            BufKinds::DamageTxPercent
            | BufKinds::DamageRxPercent
            | BufKinds::HealRxPercent
            | BufKinds::HealTxPercent => {
                let applied_value = processed_effect_param.number_of_applies * ep.buffer.value;
                self.update_buffer(&Buffer {
                    kind: ep.buffer.kind.clone(),
                    value: applied_value,
                    is_percent: true,
                    ..Default::default()
                });
                processed_effect_param.log = LogData {
                    message: format!("{} {}%", ep.buffer.kind, applied_value),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::ReinitBuf => {
                // Restart all HOTs/DOTs on the given stat
                let stats_name = ep.buffer.stats_name.clone();
                for gae in self.all_effects.iter_mut() {
                    if gae
                        .processed_effect_param
                        .input_effect_param
                        .buffer
                        .stats_name
                        == stats_name
                        && is_effet_hot_or_dot(
                            &gae.processed_effect_param.input_effect_param.buffer.kind,
                        )
                    {
                        gae.processed_effect_param.counter_turn = 0;
                    }
                }
                processed_effect_param.log = LogData {
                    message: format!("HOTs/DOTs on '{}' restarted", stats_name),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::RemoveOneDebuf => {
                // Remove the first (oldest) active debuf effect (negative value)
                if let Some(pos) = self
                    .all_effects
                    .iter()
                    .position(|gae| gae.processed_effect_param.input_effect_param.buffer.value < 0)
                {
                    self.all_effects.remove(pos);
                }
                processed_effect_param.log = LogData {
                    message: "One debuf removed".to_owned(),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::BoostHotsByPercentage => {
                // Boost all active HOT values by value%
                let boost_percent = ep.buffer.value;
                for gae in self.all_effects.iter_mut() {
                    if effect::is_hot(
                        &gae.processed_effect_param.input_effect_param.buffer.kind,
                        &gae.processed_effect_param
                            .input_effect_param
                            .buffer
                            .stats_name,
                        gae.processed_effect_param.input_effect_param.buffer.value,
                    ) {
                        let cur_val = gae.processed_effect_param.input_effect_param.buffer.value;
                        gae.processed_effect_param.input_effect_param.buffer.value +=
                            cur_val * boost_percent / 100;
                    }
                }
                processed_effect_param.log = LogData {
                    message: format!("HOTs boosted by {}%", boost_percent),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::BoostBufByHotsNumberInPercentage => {
                // Count active HOTs and add BoostedByHots buffer = count * value%
                let hot_count = self
                    .all_effects
                    .iter()
                    .filter(|gae| {
                        effect::is_hot(
                            &gae.processed_effect_param.input_effect_param.buffer.kind,
                            &gae.processed_effect_param
                                .input_effect_param
                                .buffer
                                .stats_name,
                            gae.processed_effect_param.input_effect_param.buffer.value,
                        )
                    })
                    .count() as i64;
                let bonus = hot_count * ep.buffer.value;
                self.update_buffer(&Buffer {
                    value: bonus,
                    is_percent: true,
                    kind: BufKinds::BoostedByHots,
                    ..Default::default()
                });
                processed_effect_param.log = LogData {
                    message: format!("{} HOTs => {}% heal boost", hot_count, bonus),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::BlockHealAtk => {
                self.is_heal_atk_blocked = true;
                processed_effect_param.log = LogData {
                    message: format!("Heals blocked for {} turns", ep.nb_turns),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::MultiValue => {
                // Store multiplier for heal application (read in apply_buf_debuf)
                self.update_buffer(&Buffer {
                    kind: BufKinds::MultiValue,
                    value: ep.buffer.value,
                    is_percent: ep.buffer.is_percent,
                    ..Default::default()
                });
                processed_effect_param.log = LogData {
                    message: format!("Heal multiplied by {}", ep.buffer.value),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::AddAsMuchAsHp => {
                // Enable ChangeByHealValue passive: overheal boosts the given stat
                self.update_buffer(&Buffer {
                    kind: BufKinds::ChangeByHealValue,
                    value: 0,
                    is_percent: false,
                    stats_name: ep.buffer.stats_name.clone(),
                    is_passive_enabled: true,
                    is_passive: true,
                });
                processed_effect_param.log = LogData {
                    message: format!(
                        "Overheal boosts '{}' for {} turns",
                        ep.buffer.stats_name, ep.nb_turns
                    ),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::IsDamageTxHealNeedyAlly => {
                // Enable passive: previous turn's damage TX becomes heal on most needy ally
                self.update_buffer(&Buffer {
                    kind: BufKinds::IsDamageTxHealNeedyAlly,
                    value: 0,
                    is_percent: false,
                    is_passive_enabled: true,
                    is_passive: true,
                    ..Default::default()
                });
                processed_effect_param.log = LogData {
                    message: "Previous turn damage TX => HP heal on most needy ally".to_owned(),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::PercentageIntoDamages => {
                // Stored as an effect in all_effects; conversion logic runs during heal processing
                processed_effect_param.log = LogData {
                    message: format!(
                        "{}% of '{}' heals converted to damages for {} turns",
                        ep.sub_value_effect, ep.buffer.stats_name, ep.nb_turns
                    ),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::RepeatAsManyAsPossible => {
                // number_of_applies was set by process_atk via ApplyEffectInit before this call
                processed_effect_param.log = LogData {
                    message: format!(
                        "Attack repeated {} times",
                        processed_effect_param.number_of_applies
                    ),
                    color: "".to_owned(),
                };
                // Fall through: let the normal effect application use number_of_applies
            }
            BufKinds::RepeatIfHeal => {
                // Handled in process_one_effect before reaching here
                processed_effect_param.log = LogData {
                    message: "RepeatIfHeal condition evaluated".to_owned(),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::ConditionDamagePrevTurn => {
                // Handled in process_one_effect before reaching here
                processed_effect_param.log = LogData {
                    message: "ConditionDamagePrevTurn evaluated".to_owned(),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::ChangeMaxStatByPercentage => {
                let dir = if ep.buffer.value >= 0 { "increased" } else { "decreased" };
                processed_effect_param.log = LogData {
                    message: format!(
                        "Max {} {} by {}%",
                        ep.buffer.stats_name, dir, ep.buffer.value.abs()
                    ),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::ChangeMaxStatByValue => {
                let dir = if ep.buffer.value >= 0 { "increased" } else { "decreased" };
                processed_effect_param.log = LogData {
                    message: format!(
                        "Max {} {} by {}",
                        ep.buffer.stats_name, dir, ep.buffer.value.abs()
                    ),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            BufKinds::ChangeCurrentStatByValue => {
                let dir = if ep.buffer.value >= 0 { "increased" } else { "decreased" };
                processed_effect_param.log = LogData {
                    message: format!(
                        "Current {} {} by {}",
                        ep.buffer.stats_name, dir, ep.buffer.value.abs()
                    ),
                    color: "".to_owned(),
                };
            }
            BufKinds::ChangeCurrentStatByPercentage => {
                let dir = if ep.buffer.value >= 0 { "increased" } else { "decreased" };
                processed_effect_param.log = LogData {
                    message: format!(
                        "Current {} {} by {}%",
                        ep.buffer.stats_name, dir, ep.buffer.value.abs()
                    ),
                    color: "".to_owned(),
                };
            }
            _ => {}
        }
        Ok(processed_effect_param)
    }

    pub fn process_one_effect(
        &mut self,
        ep: &EffectParam,
        atk_name: &str,
        game_state: &GameState,
        is_crit: bool,
    ) -> Result<ProcessedEffectParam> {
        let mut effect_param_mutable = ep.clone();

        // Gate condition: skip remaining effects when no damage was dealt on the previous turn
        if ep.buffer.kind == BufKinds::ConditionDamagePrevTurn {
            let prev_turn = game_state.current_turn_nb.saturating_sub(1) as u64;
            let did_damage = game_state.current_turn_nb > 0
                && self
                    .tx_rx
                    .get(AmountType::DamageTx as usize)
                    .and_then(|m| m.get(&prev_turn))
                    .map(|&v| v != 0)
                    .unwrap_or(false);
            let number_of_applies = if did_damage { 1 } else { 0 };
            return Ok(ProcessedEffectParam {
                input_effect_param: ep.clone(),
                number_of_applies,
                log: LogData {
                    message: if did_damage {
                        "Condition met: damage dealt on previous turn".to_owned()
                    } else {
                        "Condition failed: no damage on previous turn".to_owned()
                    },
                    color: "".to_owned(),
                },
                ..Default::default()
            });
        }

        // Probabilistic repeat: repeat attack with value% chance if heal was done last turn
        if ep.buffer.kind == BufKinds::RepeatIfHeal {
            let prev_turn = game_state.current_turn_nb.saturating_sub(1) as u64;
            let did_heal = game_state.current_turn_nb > 0
                && self
                    .tx_rx
                    .get(AmountType::HealTx as usize)
                    .and_then(|m| m.get(&prev_turn))
                    .map(|&v| v > 0)
                    .unwrap_or(false);
            let number_of_applies = if did_heal {
                let chance = ep.buffer.value.clamp(0, 100) as u64;
                let roll = get_random_nb(1, 100);
                if roll <= chance as i64 {
                    ep.sub_value_effect.max(1)
                } else {
                    0
                }
            } else {
                0
            };
            self.update_buffer(&Buffer {
                value: number_of_applies,
                is_percent: false,
                kind: BufKinds::ApplyEffectInit,
                ..Default::default()
            });
            return Ok(ProcessedEffectParam {
                input_effect_param: ep.clone(),
                number_of_applies,
                log: LogData {
                    message: format!(
                        "RepeatIfHeal: {} repeat(s) ({}% chance, healed_prev={})",
                        number_of_applies, ep.buffer.value, did_heal
                    ),
                    color: "".to_owned(),
                },
                ..Default::default()
            });
        }

        // Preprocess effectParam before applying it
        // update effectParam -> only used on in case of atk launched
        if is_crit && is_boosted_by_crit(&ep.buffer.kind) {
            effect_param_mutable.sub_value_effect =
                (COEFF_CRIT_STATS * ep.sub_value_effect as f64) as i64;
            effect_param_mutable.buffer.value = (COEFF_CRIT_STATS * ep.buffer.value as f64) as i64;
        }
        // conditions
        if let Some(cond) = ep
            .conditions
            .iter()
            .find(|c| c.kind == ConditionKind::NbEnnemiesDied)
        {
            effect_param_mutable.buffer.value +=
                game_state.died_ennemies[&(game_state.current_turn_nb - 1)].len() as i64
                    * cond.value;
        }

        // Process and return the new effect param
        self.process_effect_type(&effect_param_mutable, atk_name)
    }

    pub fn process_dodging(
        &mut self,
        atk_level: u64,
        class: &Class,
        current_dodge: u64,
        id_name: &str,
        drought_threshold: Option<u32>,
    ) {
        let dodge_info = if atk_level == ULTIMATE_LEVEL {
            // Ultimate attacks can never be dodged or blocked
            DodgeInfo {
                name: id_name.to_owned(),
                is_dodging: false,
                is_blocking: false,
            }
        } else {
            let effective_dodge = softcap_percent(current_dodge);
            let rand_nb = get_random_nb(1, 100);

            // Streak-breaker: if the character hasn't dodged in `threshold` turns, guarantee dodge
            let dodge_guaranteed = drought_threshold
                .map(|t| self.dodge_drought_counter >= t)
                .unwrap_or(false);

            let is_dodging =
                *class != Class::Berserker && (dodge_guaranteed || rand_nb <= effective_dodge);
            let is_blocking = *class == Class::Berserker;

            // Update drought counter
            if is_dodging {
                self.dodge_drought_counter = 0;
            } else {
                self.dodge_drought_counter += 1;
            }

            DodgeInfo {
                name: id_name.to_owned(),
                is_dodging,
                is_blocking,
            }
        };
        self.dodge_info = dodge_info;
    }

    pub fn remove_malus_effect(&mut self, ep: &EffectParam) -> Result<()> {
        match ep.buffer.kind {
            BufKinds::BlockHealAtk => self.is_heal_atk_blocked = false,
            BufKinds::DamageRxPercent
            | BufKinds::DamageTxPercent
            | BufKinds::HealRxPercent
            | BufKinds::HealTxPercent => self.update_buffer(&Buffer {
                value: -ep.buffer.value,
                is_percent: true,
                kind: ep.buffer.kind.clone(),
                is_passive: false,
                ..Default::default()
            }),
            _ => {}
        }
        Ok(())
    }

    pub fn has_buffer_type(&self, buf_type: &BufKinds) -> bool {
        self.all_buffers.iter().any(|b| b.kind == *buf_type)
    }

    pub fn get_buffer_by_type(&self, buf_type: &BufKinds) -> Option<&Buffer> {
        self.all_buffers.iter().find(|b| b.kind == *buf_type)
    }

    pub fn get_mut_buffer_by_type(&mut self, buf_type: &BufKinds) -> Option<&mut Buffer> {
        self.all_buffers.iter_mut().find(|b| b.kind == *buf_type)
    }

    pub fn update_buffer(&mut self, buffer: &Buffer) {
        // find if the buffer already exists
        if let Some(buf) = self.all_buffers.iter_mut().find(|b| b.kind == buffer.kind) {
            buf.update_buf(buffer.value, buffer.is_percent, "");
        } else {
            // else push new buffer
            self.all_buffers.push(buffer.clone());
        }
    }

    pub fn process_critical_strike(
        &mut self,
        atk: &AttackType,
        current_critical: i64,
        drought_threshold: Option<u32>,
    ) -> Result<bool> {
        // Priority 1: passive guarantee — `NextHealAtkIsCrit` fires unconditionally
        // on the next heal attack, regardless of the dice roll.
        let is_crit_by_passive =
            if let Some(buf) = self.get_buffer_by_type(&BufKinds::NextHealAtkIsCrit) {
                buf.is_passive_enabled && atk.has_only_heal_effect()
            } else {
                false
            };
        if is_crit_by_passive {
            self.crit_drought_counter = 0;
            if let Some(buf) = self.get_mut_buffer_by_type(&BufKinds::NextHealAtkIsCrit) {
                buf.is_passive_enabled = false;
            }
            return Ok(true);
        }

        // Apply hyperbolic softcap: P = stat / (100 + stat) * 100
        let effective_critical = softcap_percent(current_critical.max(0) as u64);

        // Priority 2: streak-breaker — guarantee after `threshold` consecutive non-crits
        let crit_guaranteed = drought_threshold
            .map(|t| self.crit_drought_counter >= t)
            .unwrap_or(false);

        let rand_nb = get_random_nb(1, 100);
        let is_crit = crit_guaranteed || rand_nb <= effective_critical;

        // For excess stat above raw 60: still converts to DamageCritCapped bonus
        let crit_capped = 60;
        let delta_capped = std::cmp::max(0, current_critical - crit_capped);

        if is_crit {
            self.crit_drought_counter = 0;
            if delta_capped > 0 {
                self.update_buffer(&Buffer {
                    is_passive_enabled: false,
                    value: delta_capped,
                    is_percent: false,
                    stats_name: String::new(),
                    kind: BufKinds::DamageCritCapped,
                    is_passive: false,
                });
            }
            Ok(true)
        } else {
            self.crit_drought_counter += 1;
            Ok(false)
        }
    }

    pub fn is_effect_applied(&self, target_data: &TargetData) -> bool {
        // eval effect target logic
        if !target_data.is_potential_target_on_effect() {
            return false;
        }

        // eval target choice `is_current_target`
        if (target_data.effect_param.target_kind == TARGET_ENNEMY
            || target_data.effect_param.target_kind == TARGET_ALLY)
            && target_data.effect_param.reach == INDIVIDUAL
            && !self.is_current_target
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is {} but not current target.",
                target_data.effect_param.buffer.kind,
                target_data.target_id_name,
                target_data.effect_param.target_kind
            );
            return false;
        }
        if self.is_dodging(&target_data.effect_param.target_kind)
            && target_data.target_chara_kind != target_data.launcher_chara_kind
            && self.is_current_target
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is dodging.",
                target_data.effect_param.buffer.kind,
                target_data.target_id_name
            );
            return false;
        }

        true
    }

    fn process_hot_or_dot(
        local_log: &mut Vec<LogData>,
        hot_and_dot: &mut i64,
        gae: &GameAtkEffect,
    ) {
        let amount = gae.effect_outcome.full_amount_tx;
        *hot_and_dot += amount;
        let (effect_type, color) = if amount > 0 {
            ("HOT", LIGHT_GREEN)
        } else {
            ("DOT", DARK_RED)
        };
        local_log.push(LogData {
            message: format!(
                "\u{1f7e2} {} {} HP from {}",
                effect_type, amount, gae.atk_type.name
            ),
            color: color.to_string(),
        });
    }

    pub fn process_hot_and_dot(&mut self, current_turn_nb: usize) -> (Vec<LogData>, i64) {
        let mut logs = Vec::new();
        let mut hot_and_dot = 0;
        // First process all the effects whatever their order
        for gae in self.all_effects.iter() {
            if gae.launching_turn == current_turn_nb {
                continue;
            }
            // Process hot or dot
            if gae
                .processed_effect_param
                .input_effect_param
                .buffer
                .stats_name
                == HP
                && is_effet_hot_or_dot(&gae.processed_effect_param.input_effect_param.buffer.kind)
            {
                Self::process_hot_or_dot(&mut logs, &mut hot_and_dot, gae);
            }
        }
        (logs, hot_and_dot)
    }

    pub fn add_effect_on_player(&mut self, gae: GameAtkEffect) {
        self.all_effects.push(gae);
    }

    pub fn clear(&mut self) {
        self.all_effects.clear();
        // TODO remove all buffers except passive ones -> add item to Buffer struct to know if it's a passive one or not
        self.all_buffers.clear();
        self.is_first_round = true;
        self.atk_pattern_queue.clear();
        self.is_heal_atk_blocked = false;
        self.is_random_target = false;
        self.is_current_target = false;
        self.is_potential_target = false;
        self.actions_done_in_round = 0;
        // Re-initialize with empty-but-sized slots so init_aggro_on_turn and
        // process_aggro keep working correctly after a scenario reset.
        self.tx_rx = (0..AmountType::EnumSize as usize)
            .map(|_| HashMap::new())
            .collect();
        self.crit_drought_counter = 0;
        self.dodge_drought_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character_mod::{
            buffers::{BufKinds, Buffer},
            character::Character,
            rounds_information::{CharacterRoundsInfo, HotsBufs},
            target::TargetData,
        },
        common::constants::{
            all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
            paths_const::TEST_OFFLINE_ROOT,
            stats_const::*,
        },
        server::players_manager::GameAtkEffect,
        testing::{
            testing_all_characters::testing_all_equipment,
            testing_effect::{
                build_buf_effect_individual, build_cooldown_effect, build_debuf_effect_individual,
                build_dmg_effect_individual, build_dot_effect_individual, build_dot_effect_zone,
                build_hot_effect_all, build_hot_effect_individual, build_hot_effect_zone,
            },
        },
    };

    #[test]
    fn unit_get_hot_and_buf_nbs() {
        use crate::character_mod::{attack_type::AttackType, effect::EffectOutcome};

        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&vec![]);
        assert_eq!(result, HotsBufs::default());
        let mut all_effects: Vec<GameAtkEffect> = vec![];
        // add a 1-turn-effect (nb_turns < 2, should be ignored)
        all_effects.push(GameAtkEffect {
            processed_effect_param: build_dmg_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(result, HotsBufs::default());

        // add a 2-turn HOT: +30 HP
        all_effects.push(GameAtkEffect {
            processed_effect_param: build_hot_effect_individual(),
            atk_type: AttackType {
                name: "TestHot".to_owned(),
                ..Default::default()
            },
            effect_outcome: EffectOutcome {
                full_amount_tx: 30,
                ..Default::default()
            },
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                hot_txt: vec!["TestHot: 30 HP × 2 turns".to_owned()],
                ..Default::default()
            }
        );

        // add a 3-turn DOT: -20 HP
        all_effects.push(GameAtkEffect {
            processed_effect_param: build_dot_effect_individual(),
            atk_type: AttackType {
                name: "TestDot".to_owned(),
                ..Default::default()
            },
            effect_outcome: EffectOutcome {
                full_amount_tx: -20,
                ..Default::default()
            },
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                hot_txt: vec!["TestHot: 30 HP × 2 turns".to_owned()],
                dot_txt: vec!["TestDot: 20 HP × 3 turns".to_owned()],
                ..Default::default()
            }
        );

        // add a 3-turn buff: +20 Magical armor (non-HP)
        all_effects.push(GameAtkEffect {
            processed_effect_param: build_buf_effect_individual(),
            atk_type: AttackType {
                name: "TestBuf".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 1,
                hot_txt: vec!["TestHot: 30 HP × 2 turns".to_owned()],
                dot_txt: vec!["TestDot: 20 HP × 3 turns".to_owned()],
                buf_txt: vec![format!("TestBuf: 20 {} × 3 turns", MAGICAL_ARMOR)],
                ..Default::default()
            }
        );

        // add a 3-turn debuff: -20 Magical armor (non-HP)
        all_effects.push(GameAtkEffect {
            processed_effect_param: build_debuf_effect_individual(),
            atk_type: AttackType {
                name: "TestDebuf".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 1,
                debuf_nb: 1,
                hot_txt: vec!["TestHot: 30 HP × 2 turns".to_owned()],
                dot_txt: vec!["TestDot: 20 HP × 3 turns".to_owned()],
                buf_txt: vec![format!("TestBuf: 20 {} × 3 turns", MAGICAL_ARMOR)],
                debuf_txt: vec![format!("TestDebuf: 20 {} × 3 turns", MAGICAL_ARMOR)],
            }
        );
    }

    #[test]
    fn unit_apply_buf_debuf() {
        let mut cri = CharacterRoundsInfo::default();
        // no buf/debuf
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        assert_eq!(result, 100);

        // buf defub damage against ennemy

        // Launcher TX: BufTypes::DamageTxPercent
        // damage buf aigainst ennemy
        cri.update_buffer(&Buffer {
            kind: BufKinds::DamageTxPercent,
            value: 20,
            is_percent: false,
            stats_name: String::new(),
            is_passive_enabled: false,
            is_passive: false,
        });
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, false);
        // -100 -20 = -120
        assert_eq!(result, -120);
        // same but with critical strike
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 -20 = -120 * 2 = -240
        assert_eq!(result, -240);
        cri.reset_all_buffers();

        //Receiver RX: BufTypes::DamageRx
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 20,
            is_percent: false,
            stats_name: String::new(),
            kind: BufKinds::DamageRxPercent,
            is_passive: false,
        });
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, false);
        // -100 -20 = -120
        assert_eq!(result, -120);
        cri.reset_all_buffers();

        //Receiver RX: BufTypes::DamageCritCapped
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 2,
            is_percent: false,
            stats_name: String::new(),
            kind: BufKinds::DamageCritCapped,
            is_passive: false,
        });
        // crit is doubled init:2 -> 2 + 2 = 4
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 * 4 = -400
        assert_eq!(result, -400);

        // it can be accumulated with damage buf
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 20,
            is_percent: false,
            stats_name: String::new(),
            kind: BufKinds::DamageTxPercent,
            is_passive: false,
        });
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 -20 = -120* 4 = -480
        assert_eq!(result, -480);
        cri.reset_all_buffers();

        // buf debuf heal against ally

        // Launcher TX: BufTypes::MultiValue
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 3,
            is_percent: false,
            stats_name: String::new(),
            kind: BufKinds::MultiValue,
            is_passive: false,
        });
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 * 3 = 300
        assert_eq!(result, 300);
        cri.reset_all_buffers();

        // BufTypes::HealTx
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 20,
            is_percent: false,
            stats_name: String::new(),
            kind: BufKinds::HealTxPercent,
            is_passive: false,
        });
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.reset_all_buffers();
        // BufTypes::HealRx
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 20,
            is_percent: false,
            stats_name: String::new(),
            kind: BufKinds::HealRxPercent,
            is_passive: false,
        });
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.reset_all_buffers();
        // BufTypes::BoostedByHots
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 20,
            is_percent: false,
            stats_name: String::new(),
            kind: BufKinds::BoostedByHots,
            is_passive: false,
        });
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.reset_all_buffers();
    }

    #[test]
    fn unit_reset_all_buffers() {
        let mut cri = CharacterRoundsInfo::default();
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 20,
            is_percent: false,
            stats_name: String::new(),
            kind: BufKinds::DamageTxPercent,
            is_passive: false,
        });
        cri.reset_all_buffers();
        assert!(cri.all_buffers.is_empty());
    }

    #[test]
    fn unit_update_buf() {
        let mut cri = CharacterRoundsInfo::default();
        cri.update_buffer(&Buffer {
            is_passive_enabled: false,
            value: 20,
            is_percent: false,
            stats_name: HP.to_owned(),
            kind: BufKinds::DamageTxPercent,
            is_passive: false,
        });
        assert_eq!(
            20,
            cri.get_buffer_by_type(&BufKinds::DamageTxPercent)
                .as_ref()
                .unwrap()
                .value
        );
        assert!(
            !cri.get_buffer_by_type(&BufKinds::DamageTxPercent)
                .as_ref()
                .unwrap()
                .is_percent
        );
        assert_eq!(
            HP,
            cri.get_buffer_by_type(&BufKinds::DamageTxPercent)
                .as_ref()
                .unwrap()
                .stats_name
        );
    }

    #[test]
    fn unit_is_effect_applied() {
        let c1 = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        let mut c2 = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        c2.db_full_name = "other".to_string();
        c2.id_name = "other_#1".to_string();
        let mut boss1 = Character::try_new_from_json(
            "./tests/offlines/characters/test_boss1.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        // effect on himself
        let mut target_data_1 = TargetData {
            launcher_id_name: c1.id_name.clone(),
            target_id_name: c1.id_name.clone(),
            target_chara_kind: c1.kind.clone(),
            launcher_chara_kind: c1.kind.clone(),
            effect_param: build_cooldown_effect().input_effect_param,
        };
        let mut target_data_2 = TargetData {
            launcher_id_name: c1.id_name.clone(),
            target_id_name: c2.id_name.clone(),
            target_chara_kind: c2.kind.clone(),
            launcher_chara_kind: c1.kind.clone(),
            effect_param: build_cooldown_effect().input_effect_param,
        };
        let mut target_data_3 = TargetData {
            launcher_id_name: c1.id_name.clone(),
            target_id_name: boss1.id_name.clone(),
            target_chara_kind: boss1.kind.clone(),
            launcher_chara_kind: c1.kind.clone(),
            effect_param: build_cooldown_effect().input_effect_param,
        };
        assert!(c1.character_rounds_info.is_effect_applied(&target_data_1));
        // other ally
        assert!(!c2.character_rounds_info.is_effect_applied(&target_data_2));
        // boss
        assert!(
            !boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );

        // effect on ally individual
        target_data_1.effect_param = build_hot_effect_individual().input_effect_param;
        target_data_2.effect_param = build_hot_effect_individual().input_effect_param;
        target_data_3.effect_param = build_hot_effect_individual().input_effect_param;
        // target is himself
        assert!(!c1.character_rounds_info.is_effect_applied(&target_data_1));
        // other ally
        // not targeted on main atk
        c2.character_rounds_info.is_current_target = false;
        assert!(!c2.character_rounds_info.is_effect_applied(&target_data_2));
        // targeted on main atk
        c2.character_rounds_info.is_current_target = true;
        assert!(c2.character_rounds_info.is_effect_applied(&target_data_2));
        // boss
        assert!(
            !boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );

        // effect on ennemy individual
        target_data_1.effect_param = build_dmg_effect_individual().input_effect_param;
        target_data_2.effect_param = build_dmg_effect_individual().input_effect_param;
        target_data_3.effect_param = build_dmg_effect_individual().input_effect_param;
        assert!(!c1.character_rounds_info.is_effect_applied(&target_data_1));
        // other ally
        assert!(!c2.character_rounds_info.is_effect_applied(&target_data_2));
        // boss
        // targeted on main atk
        boss1.character_rounds_info.is_current_target = true;
        assert!(
            boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );
        // not targeted on main atk
        boss1.character_rounds_info.is_current_target = false;
        assert!(
            !boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );

        // effect on ally ZONE
        target_data_1.effect_param = build_hot_effect_zone().input_effect_param;
        target_data_2.effect_param = build_hot_effect_zone().input_effect_param;
        target_data_3.effect_param = build_hot_effect_zone().input_effect_param;
        // target is himself
        assert!(!c1.character_rounds_info.is_effect_applied(&target_data_1));
        // other ally
        // targeted on main atk
        assert!(c2.character_rounds_info.is_effect_applied(&target_data_2));
        // boss
        assert!(
            !boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );

        // effect on ennemy ZONE
        target_data_1.effect_param = build_dot_effect_zone().input_effect_param;
        target_data_2.effect_param = build_dot_effect_zone().input_effect_param;
        target_data_3.effect_param = build_dot_effect_zone().input_effect_param;
        // target is himself
        assert!(!c1.character_rounds_info.is_effect_applied(&target_data_1));
        // other ally
        assert!(!c2.character_rounds_info.is_effect_applied(&target_data_2));
        // boss
        // targeted on main atk
        boss1.character_rounds_info.is_current_target = true;
        assert!(
            boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );
        // not targeted on main atk
        boss1.character_rounds_info.is_current_target = false;
        assert!(
            boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );

        // effect on all allies
        target_data_1.effect_param = build_hot_effect_all().input_effect_param;
        target_data_2.effect_param = build_hot_effect_all().input_effect_param;
        target_data_3.effect_param = build_hot_effect_all().input_effect_param;
        // target is himself
        assert!(c1.character_rounds_info.is_effect_applied(&target_data_1));
        assert!(c1.character_rounds_info.is_effect_applied(&target_data_1));
        // other ally
        assert!(c2.character_rounds_info.is_effect_applied(&target_data_2));
        assert!(c2.character_rounds_info.is_effect_applied(&target_data_2));
        // boss
        // targeted on main atk
        boss1.character_rounds_info.is_current_target = true;
        assert!(
            !boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );
        boss1.character_rounds_info.is_current_target = false;
        assert!(
            !boss1
                .character_rounds_info
                .is_effect_applied(&target_data_3)
        );
    }

    #[test]
    fn unit_add_exp() {
        let mut cri = CharacterRoundsInfo::default();
        let level_up = cri.add_exp(98);
        assert_eq!(cri.exp, 98);
        assert!(!level_up);
        let level_up = cri.add_exp(5);
        assert_eq!(cri.exp, 3);
        assert!(level_up);
    }

    #[test]
    fn unit_has_buffer_type() {
        let mut cri = CharacterRoundsInfo::default();
        assert!(!cri.has_buffer_type(&BufKinds::DamageTxPercent));
        cri.update_buffer(&Buffer {
            kind: BufKinds::DamageTxPercent,
            ..Default::default()
        });
        assert!(cri.has_buffer_type(&BufKinds::DamageTxPercent));
        assert!(!cri.has_buffer_type(&BufKinds::HealTxPercent));
    }

    #[test]
    fn unit_increment_counter_effect() {
        let mut cri = CharacterRoundsInfo::default();
        // no effects: no-op
        cri.increment_counter_effect();
        assert!(cri.all_effects.is_empty());

        cri.all_effects.push(GameAtkEffect {
            processed_effect_param: build_hot_effect_individual(),
            ..Default::default()
        });
        cri.all_effects.push(GameAtkEffect {
            processed_effect_param: build_dot_effect_individual(),
            ..Default::default()
        });
        assert_eq!(0, cri.all_effects[0].processed_effect_param.counter_turn);
        assert_eq!(0, cri.all_effects[1].processed_effect_param.counter_turn);

        cri.increment_counter_effect();
        assert_eq!(1, cri.all_effects[0].processed_effect_param.counter_turn);
        assert_eq!(1, cri.all_effects[1].processed_effect_param.counter_turn);

        cri.increment_counter_effect();
        assert_eq!(2, cri.all_effects[0].processed_effect_param.counter_turn);
        assert_eq!(2, cri.all_effects[1].processed_effect_param.counter_turn);
    }

    #[test]
    fn unit_process_hot_and_dot() {
        let mut cri = CharacterRoundsInfo::default();
        // empty effects
        let (logs, total) = cri.process_hot_and_dot(0);
        assert_eq!(0, total);
        assert!(logs.is_empty());

        // effect launched this turn => ignored
        let mut hot = GameAtkEffect {
            processed_effect_param: build_hot_effect_individual(),
            launching_turn: 1,
            ..Default::default()
        };
        cri.all_effects.push(hot.clone());
        let (logs, total) = cri.process_hot_and_dot(1);
        assert_eq!(0, total);
        assert!(logs.is_empty());

        // effect from a previous turn => applied
        hot.launching_turn = 0;
        cri.all_effects.clear();
        let hot_value = hot.processed_effect_param.input_effect_param.buffer.value;
        hot.effect_outcome.full_amount_tx = hot_value;
        cri.all_effects.push(hot);

        let mut dot = GameAtkEffect {
            processed_effect_param: build_dot_effect_individual(),
            launching_turn: 0,
            ..Default::default()
        };
        let dot_value = dot.processed_effect_param.input_effect_param.buffer.value;
        dot.effect_outcome.full_amount_tx = dot_value;
        cri.all_effects.push(dot);

        let (logs, total) = cri.process_hot_and_dot(1);
        assert_eq!(hot_value + dot_value, total);
        assert_eq!(2, logs.len());
    }

    #[test]
    fn unit_clear() {
        let mut cri = CharacterRoundsInfo::default();
        cri.all_effects.push(GameAtkEffect::default());
        cri.update_buffer(&Buffer {
            kind: BufKinds::DamageTxPercent,
            ..Default::default()
        });
        cri.is_heal_atk_blocked = true;
        cri.is_random_target = true;
        cri.is_current_target = true;
        cri.is_potential_target = true;
        cri.actions_done_in_round = 5;
        cri.tx_rx.push(Default::default());

        cri.clear();

        assert!(cri.all_effects.is_empty());
        assert!(cri.all_buffers.is_empty());
        assert!(cri.is_first_round);
        assert!(!cri.is_heal_atk_blocked);
        assert!(!cri.is_random_target);
        assert!(!cri.is_current_target);
        assert!(!cri.is_potential_target);
        assert_eq!(0, cri.actions_done_in_round);
        // After clear(), tx_rx is re-initialized with AmountType::EnumSize empty slots
        // so that aggro and damage tracking keep working on the next scenario.
        assert_eq!(AmountType::EnumSize as usize, cri.tx_rx.len());
        assert!(cri.tx_rx.iter().all(|m| m.is_empty()));
    }

    use crate::testing::testing_effect::build_change_max_hp_by_percent_effect;

    #[test]
    fn unit_change_max_hp_by_percent_is_hot() {
        // ChangeMaxStatByPercentage on HP with positive value (Cor d'Erebor style)
        // must be classified as HOT, not DOT.
        use crate::character_mod::attack_type::AttackType;
        let all_effects = vec![GameAtkEffect {
            processed_effect_param: build_change_max_hp_by_percent_effect(),
            atk_type: AttackType {
                name: "Cor d'Erebor".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }];
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result.hot_nb, 1,
            "ChangeMaxStatByPercentage +HP should be HOT"
        );
        assert_eq!(
            result.dot_nb, 0,
            "ChangeMaxStatByPercentage +HP should not be DOT"
        );
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Tests for newly-implemented BufKinds in process_effect_type
    // ──────────────────────────────────────────────────────────────────────────

    use crate::character_mod::effect::{EffectParam, ProcessedEffectParam};
    use crate::character_mod::rounds_information::AmountType;
    use crate::server::game_state::GameState;

    fn make_ep(kind: BufKinds, value: i64, stats_name: &str, nb_turns: i64) -> EffectParam {
        EffectParam {
            nb_turns,
            buffer: Buffer {
                kind,
                value,
                stats_name: stats_name.to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_hot_gae(value: i64) -> GameAtkEffect {
        GameAtkEffect {
            processed_effect_param: ProcessedEffectParam {
                input_effect_param: EffectParam {
                    nb_turns: 3,
                    buffer: Buffer {
                        kind: BufKinds::ChangeCurrentStatByValue,
                        value,
                        stats_name: HP.to_owned(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                number_of_applies: 1,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_debuf_gae(value: i64) -> GameAtkEffect {
        GameAtkEffect {
            processed_effect_param: ProcessedEffectParam {
                input_effect_param: EffectParam {
                    nb_turns: 3,
                    buffer: Buffer {
                        kind: BufKinds::ChangeCurrentStatByValue,
                        value,
                        stats_name: HP.to_owned(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                number_of_applies: 1,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn unit_process_effect_damage_tx_percent() {
        let mut cri = CharacterRoundsInfo::default();
        let ep = make_ep(BufKinds::DamageTxPercent, 15, "", 1);
        let result = cri.process_effect_type(&ep, "test_atk").unwrap();
        assert_eq!(result.number_of_applies, 1);
        let buf = cri.get_buffer_by_type(&BufKinds::DamageTxPercent).unwrap();
        assert_eq!(buf.value, 15);
        assert!(buf.is_percent);
    }

    #[test]
    fn unit_process_effect_damage_rx_percent() {
        let mut cri = CharacterRoundsInfo::default();
        let ep = make_ep(BufKinds::DamageRxPercent, 10, "", 1);
        let result = cri.process_effect_type(&ep, "test_atk").unwrap();
        assert_eq!(result.number_of_applies, 1);
        let buf = cri.get_buffer_by_type(&BufKinds::DamageRxPercent).unwrap();
        assert_eq!(buf.value, 10);
        assert!(buf.is_percent);
    }

    #[test]
    fn unit_process_effect_heal_tx_percent() {
        let mut cri = CharacterRoundsInfo::default();
        let ep = make_ep(BufKinds::HealTxPercent, 20, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        let buf = cri.get_buffer_by_type(&BufKinds::HealTxPercent).unwrap();
        assert_eq!(buf.value, 20);
        assert!(buf.is_percent);
    }

    #[test]
    fn unit_process_effect_heal_rx_percent() {
        let mut cri = CharacterRoundsInfo::default();
        let ep = make_ep(BufKinds::HealRxPercent, 25, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        let buf = cri.get_buffer_by_type(&BufKinds::HealRxPercent).unwrap();
        assert_eq!(buf.value, 25);
        assert!(buf.is_percent);
    }

    #[test]
    fn unit_process_effect_reinit_buf() {
        let mut cri = CharacterRoundsInfo::default();
        // Add an active HOT with counter_turn = 2
        let mut gae = make_hot_gae(30);
        gae.processed_effect_param.counter_turn = 2;
        cri.all_effects.push(gae);
        // ReinitBuf on HP resets the counter
        let ep = make_ep(BufKinds::ReinitBuf, 0, HP, 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        assert_eq!(0, cri.all_effects[0].processed_effect_param.counter_turn);
    }

    #[test]
    fn unit_process_effect_reinit_buf_no_match() {
        let mut cri = CharacterRoundsInfo::default();
        let mut gae = make_hot_gae(30);
        gae.processed_effect_param.counter_turn = 2;
        cri.all_effects.push(gae);
        // ReinitBuf on a different stat — should not reset HP HOT
        let ep = make_ep(BufKinds::ReinitBuf, 0, MANA, 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        assert_eq!(2, cri.all_effects[0].processed_effect_param.counter_turn);
    }

    #[test]
    fn unit_process_effect_remove_one_debuf() {
        let mut cri = CharacterRoundsInfo::default();
        cri.all_effects.push(make_hot_gae(30)); // positive — NOT a debuf
        cri.all_effects.push(make_debuf_gae(-20)); // negative — IS a debuf
        assert_eq!(2, cri.all_effects.len());
        let ep = make_ep(BufKinds::RemoveOneDebuf, 0, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        // Only the debuf removed
        assert_eq!(1, cri.all_effects.len());
        assert_eq!(
            30,
            cri.all_effects[0]
                .processed_effect_param
                .input_effect_param
                .buffer
                .value
        );
    }

    #[test]
    fn unit_process_effect_remove_one_debuf_none() {
        let mut cri = CharacterRoundsInfo::default();
        cri.all_effects.push(make_hot_gae(30));
        let ep = make_ep(BufKinds::RemoveOneDebuf, 0, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        // No debuf to remove — HOT should remain
        assert_eq!(1, cri.all_effects.len());
    }

    #[test]
    fn unit_process_effect_boost_hots_by_percentage() {
        let mut cri = CharacterRoundsInfo::default();
        cri.all_effects.push(make_hot_gae(100));
        // Boost HOTs by 20%
        let ep = make_ep(BufKinds::BoostHotsByPercentage, 20, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        // 100 + (100 * 20 / 100) = 120
        assert_eq!(
            120,
            cri.all_effects[0]
                .processed_effect_param
                .input_effect_param
                .buffer
                .value
        );
    }

    #[test]
    fn unit_process_effect_boost_hots_no_hots() {
        let mut cri = CharacterRoundsInfo::default();
        // Only a DOT: value < 0, not a HOT
        cri.all_effects.push(make_debuf_gae(-50));
        let ep = make_ep(BufKinds::BoostHotsByPercentage, 20, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        // DOT should be unchanged
        assert_eq!(
            -50,
            cri.all_effects[0]
                .processed_effect_param
                .input_effect_param
                .buffer
                .value
        );
    }

    #[test]
    fn unit_process_effect_boost_buf_by_hots_number() {
        let mut cri = CharacterRoundsInfo::default();
        cri.all_effects.push(make_hot_gae(30));
        cri.all_effects.push(make_hot_gae(40));
        // 2 HOTs × 10% = 20% boost stored in BoostedByHots
        let ep = make_ep(BufKinds::BoostBufByHotsNumberInPercentage, 10, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        let buf = cri.get_buffer_by_type(&BufKinds::BoostedByHots).unwrap();
        assert_eq!(20, buf.value);
        assert!(buf.is_percent);
    }

    #[test]
    fn unit_process_effect_boost_buf_by_hots_number_no_hots() {
        let mut cri = CharacterRoundsInfo::default();
        // 0 HOTs → 0% boost
        let ep = make_ep(BufKinds::BoostBufByHotsNumberInPercentage, 10, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        let buf = cri.get_buffer_by_type(&BufKinds::BoostedByHots).unwrap();
        assert_eq!(0, buf.value);
    }

    #[test]
    fn unit_process_effect_block_heal_atk() {
        let mut cri = CharacterRoundsInfo::default();
        assert!(!cri.is_heal_atk_blocked);
        let ep = make_ep(BufKinds::BlockHealAtk, 0, "", 3);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        assert!(cri.is_heal_atk_blocked);
    }

    #[test]
    fn unit_process_effect_multi_value() {
        let mut cri = CharacterRoundsInfo::default();
        let ep = make_ep(BufKinds::MultiValue, 3, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        let buf = cri.get_buffer_by_type(&BufKinds::MultiValue).unwrap();
        assert_eq!(3, buf.value);
    }

    #[test]
    fn unit_process_effect_add_as_much_as_hp() {
        let mut cri = CharacterRoundsInfo::default();
        let ep = make_ep(BufKinds::AddAsMuchAsHp, 0, MAGICAL_POWER, 3);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        let buf = cri
            .get_buffer_by_type(&BufKinds::ChangeByHealValue)
            .unwrap();
        assert!(buf.is_passive_enabled);
        assert_eq!(MAGICAL_POWER, buf.stats_name);
    }

    #[test]
    fn unit_process_effect_is_damage_tx_heal_needy_ally() {
        let mut cri = CharacterRoundsInfo::default();
        let ep = make_ep(BufKinds::IsDamageTxHealNeedyAlly, 0, "", 1);
        cri.process_effect_type(&ep, "test_atk").unwrap();
        let buf = cri
            .get_buffer_by_type(&BufKinds::IsDamageTxHealNeedyAlly)
            .unwrap();
        assert!(buf.is_passive_enabled);
    }

    #[test]
    fn unit_process_effect_percentage_into_damages() {
        let mut cri = CharacterRoundsInfo::default();
        let mut ep = make_ep(BufKinds::PercentageIntoDamages, 0, HP, 5);
        ep.sub_value_effect = 50;
        let result = cri.process_effect_type(&ep, "test_atk").unwrap();
        assert!(result.log.message.contains("50%"));
        assert!(result.log.message.contains(HP));
    }

    #[test]
    fn unit_process_effect_repeat_as_many_as_possible() {
        let mut cri = CharacterRoundsInfo::default();
        // Pre-set ApplyEffectInit to 4 (as process_atk would do)
        cri.update_buffer(&Buffer {
            kind: BufKinds::ApplyEffectInit,
            value: 4,
            ..Default::default()
        });
        let ep = make_ep(BufKinds::RepeatAsManyAsPossible, -50, HP, 1);
        let result = cri.process_effect_type(&ep, "test_atk").unwrap();
        assert_eq!(4, result.number_of_applies);
    }

    #[test]
    fn unit_process_one_effect_condition_damage_prev_turn_met() {
        let mut cri = CharacterRoundsInfo::default();
        // Ensure tx_rx has enough slots
        for _ in 0..AmountType::EnumSize as usize {
            cri.tx_rx.push(std::collections::HashMap::new());
        }
        // Damage TX on turn 0
        cri.tx_rx[AmountType::DamageTx as usize].insert(0, 100);
        let gs = GameState {
            current_turn_nb: 1,
            ..Default::default()
        }; // prev turn = 0

        let ep = EffectParam {
            nb_turns: 1,
            buffer: Buffer {
                kind: BufKinds::ConditionDamagePrevTurn,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = cri.process_one_effect(&ep, "test", &gs, false).unwrap();
        assert_eq!(1, result.number_of_applies);
    }

    #[test]
    fn unit_process_one_effect_condition_damage_prev_turn_failed() {
        let mut cri = CharacterRoundsInfo::default();
        for _ in 0..AmountType::EnumSize as usize {
            cri.tx_rx.push(std::collections::HashMap::new());
        }
        // No damage TX on any turn
        let gs = GameState {
            current_turn_nb: 1,
            ..Default::default()
        };

        let ep = EffectParam {
            nb_turns: 1,
            buffer: Buffer {
                kind: BufKinds::ConditionDamagePrevTurn,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = cri.process_one_effect(&ep, "test", &gs, false).unwrap();
        assert_eq!(0, result.number_of_applies);
        assert!(result.log.message.contains("failed"));
    }

    #[test]
    fn unit_process_effect_cooldown_uses_buffer_value() {
        let mut cri = CharacterRoundsInfo::default();
        // buffer.value=7 is the single source of truth for cooldown duration
        let ep = make_ep(BufKinds::CooldownTurnsNumber, 7, "", 1);
        let result = cri.process_effect_type(&ep, "my_atk").unwrap();
        assert!(
            result.log.message.contains("7 turns"),
            "Message: {}",
            result.log.message
        );
    }
}
