use anyhow::{Result, bail};

use crate::{
    attack_type::AttackType,
    buffers::{BufTypes, Buffers, update_damage_by_buf, update_heal_by_multi},
    character::Class,
    common::{
        all_target_const::TARGET_ENNEMY,
        attak_const::{COEFF_CRIT_DMG, COEFF_CRIT_STATS},
        character_const::ULTIMATE_LEVEL,
        effect_const::*,
        stats_const::HP,
    },
    effect::{
        self, EffectParam, ProcessedEffectParam, is_boosted_by_crit, process_decrease_on_turn,
    },
    game_manager::LogData,
    game_state::GameState,
    players_manager::{DodgeInfo, GameAtkEffects},
    target::is_target_ally,
    utils::get_random_nb,
};
use std::collections::HashMap;

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
    pub all_buffers: Vec<Buffers>,
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
    pub all_effects: Vec<GameAtkEffects>,
}

impl Default for CharacterRoundsInfo {
    fn default() -> Self {
        let mut info = CharacterRoundsInfo {
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
        };
        // init all_buffers
        info.new_buffers();

        info
    }
}

impl CharacterRoundsInfo {
    pub fn apply_launchable_atks(&mut self, launchable_atks: Vec<AttackType>) {
        self.launchable_atks = launchable_atks;
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
    /// Init all buffers with default values
    pub fn new_buffers(&mut self) {
        for _ in 0..BufTypes::EnumSize as usize {
            self.all_buffers.push(Buffers::default());
        }
    }

    /// Output: hot, dot, buf, debuf
    pub fn get_hot_and_buf_nbs_txts(all_effects: &Vec<GameAtkEffects>) -> HotsBufs {
        let mut hots_bufs = HotsBufs::default();
        for e in all_effects {
            if e.all_atk_effects.input_effect_param.nb_turns < 2 {
                continue;
            }
            let txt = Self::get_hot_and_buf_texts(&e.all_atk_effects.input_effect_param);
            if effect::is_hot(
                &e.all_atk_effects.input_effect_param.effect_type,
                &e.all_atk_effects.input_effect_param.stats_name,
                e.all_atk_effects.input_effect_param.value,
            ) {
                hots_bufs.hot_nb += 1;
                hots_bufs.hot_txt.push(txt);
            } else if e.all_atk_effects.input_effect_param.stats_name == HP {
                hots_bufs.dot_nb += 1;
                hots_bufs.dot_txt.push(txt);
            } else if e.all_atk_effects.input_effect_param.value > 0 {
                hots_bufs.buf_nb += 1;
                hots_bufs.buf_txt.push(txt);
            } else {
                hots_bufs.debuf_nb += 1;
                hots_bufs.debuf_txt.push(txt);
            }
        }
        hots_bufs
    }

    fn get_hot_and_buf_texts(ep: &EffectParam) -> String {
        if ep.stats_name.is_empty() {
            format!("{}: {}", ep.effect_type, ep.value)
        } else {
            format!("{}-{}: {}", ep.effect_type, ep.stats_name, ep.value)
        }
    }

    pub fn is_dodging(&self, target_kind: &str) -> bool {
        self.dodge_info.is_dodging && target_kind == TARGET_ENNEMY
    }

    pub fn is_blocking(&mut self, ep: &EffectParam) -> bool {
        self.dodge_info.is_blocking && ep.stats_name == HP && ep.target_kind == TARGET_ENNEMY
    }

    pub fn apply_buf_debuf(&self, full_amount: i64, target: &str, is_crit: bool) -> i64 {
        let mut real_amount = full_amount;
        let mut buf_debuf = 0;
        let mut coeff_crit = COEFF_CRIT_DMG;
        // buf debuf heal
        if full_amount > 0 && is_target_ally(target) {
            // Launcher TX: BufTypes::MultiValue
            // To place first
            if let Some(buf_multi) = self.all_buffers.get(BufTypes::MultiValue as usize)
                && buf_multi.value > 0
            {
                real_amount = update_heal_by_multi(real_amount, buf_multi.value);
            }
            // Launcher TX: BufTypes::HealTx
            if let Some(buf_hp_tx) = self.all_buffers.get(BufTypes::HealTx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_hp_tx.value, buf_hp_tx.is_percent, real_amount);
            }
            // Receiver RX: BufTypes::HealRx
            if let Some(buf_hp_rx) = self.all_buffers.get(BufTypes::HealRx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_hp_rx.value, buf_hp_rx.is_percent, real_amount);
            }
            // Launcher TX: BufTypes::BoostedByHots
            if let Some(buf_nb_hots) = self.all_buffers.get(BufTypes::BoostedByHots as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_nb_hots.value, buf_nb_hots.is_percent, real_amount);
            }
        }
        // buf debuf damage
        if full_amount < 0 && !is_target_ally(target) {
            // Launcher TX: BufTypes::DamageTx
            if let Some(buf_dmg_tx) = self.all_buffers.get(BufTypes::DamageTx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_dmg_tx.value, buf_dmg_tx.is_percent, real_amount);
            }
            // Receiver RX: BufTypes::DamageRx
            if let Some(buf_dmg_rx) = self.all_buffers.get(BufTypes::DamageRx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_dmg_rx.value, buf_dmg_rx.is_percent, real_amount);
            }
            // Receiver RX: BufTypes::DamageCritCapped
            if let Some(buf_dmg_crit) = self.all_buffers.get(BufTypes::DamageCritCapped as usize) {
                // improve crit coeff
                coeff_crit += buf_dmg_crit.value as f64;
            }
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
        self.all_buffers.iter_mut().for_each(|b| {
            b.set_buffers(0, false);
            b.is_passive_enabled = false;
        });
    }

    pub fn update_buf(
        &mut self,
        buf_type: &BufTypes,
        value: i64,
        is_percent: bool,
        stat: &str,
    ) -> Result<()> {
        if let Some(buf) = self.all_buffers.get_mut(buf_type.clone() as usize) {
            buf.update_buf(value, is_percent, stat);
            Ok(())
        } else {
            bail!("Buffer type {:?} cannot be found", buf_type);
        }
    }

    pub fn increment_counter_effect(&mut self) {
        for gae in self.all_effects.iter_mut() {
            gae.all_atk_effects.counter_turn += 1;
        }
    }

    /// Update all the bufs
    pub fn process_effect_type(
        &mut self,
        ep: &EffectParam,
        atk: &AttackType,
    ) -> Result<ProcessedEffectParam> {
        let mut processed_effect_param = ProcessedEffectParam {
            input_effect_param: ep.clone(),
            ..Default::default()
        };
        processed_effect_param.number_of_applies = 1;
        let bug_apply_init = &self.all_buffers[BufTypes::ApplyEffectInit as usize];
        if bug_apply_init.value > 0 {
            processed_effect_param.number_of_applies = bug_apply_init.value;
        }

        match ep.effect_type.as_str() {
            EFFECT_NB_COOL_DOWN => {
                processed_effect_param.log = LogData {
                    message: format!("Cooldown actif sur {} de {} tours.", atk.name, ep.nb_turns),
                    color: "".to_owned(),
                };
                return Ok(processed_effect_param);
            }
            EFFECT_NB_DECREASE_ON_TURN => {
                processed_effect_param.number_of_applies = process_decrease_on_turn(ep);
                self.update_buf(
                    &BufTypes::ApplyEffectInit,
                    processed_effect_param.number_of_applies,
                    false,
                    "",
                )?;
                processed_effect_param.log = LogData {
                    message: format!(
                        "L'attaque sera effectuée {} fois.",
                        processed_effect_param.number_of_applies
                    ),
                    color: "".to_owned(),
                };
            }
            EFFECT_REINIT => {}
            _ => {}
        }
        // Must be filled before changing value of nbTurns
        if ep.effect_type == EFFECT_REINIT {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_DELETE_BAD {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_IMPROVE_HOTS {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_BOOSTED_BY_HOTS {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_RX_BY_PERCENT {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_TX_BY_PERCENT {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE {
            processed_effect_param.log = LogData {
                message: format!("Max stat of {} is up by {}%", ep.stats_name, ep.value),
                color: "".to_owned(),
            };
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_IMPROVE_MAX_STAT_BY_VALUE {
            processed_effect_param.log = LogData {
                message: format!("Max stat of {} is up by value:{}", ep.stats_name, ep.value),
                color: "".to_owned(),
            };
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_REPEAT_AS_MANY_AS {
            // TODO
            return Ok(processed_effect_param);
        }
        if ep.effect_type == EFFECT_INTO_DAMAGE {
            // TODO
            return Ok(processed_effect_param);
        }
        Ok(processed_effect_param)
    }

    pub fn process_one_effect(
        &mut self,
        ep: &EffectParam,
        atk: &AttackType,
        game_state: &GameState,
        is_crit: bool,
    ) -> Result<ProcessedEffectParam> {
        let mut effect_param_mutable = ep.clone();

        // Preprocess effectParam before applying it
        // update effectParam -> only used on in case of atk launched
        if is_crit && is_boosted_by_crit(&ep.effect_type) {
            effect_param_mutable.sub_value_effect =
                (COEFF_CRIT_STATS * ep.sub_value_effect as f64) as i64;
            effect_param_mutable.value = (COEFF_CRIT_STATS * ep.value as f64) as i64;
        }
        // conditions
        if ep.effect_type == CONDITION_ENNEMIES_DIED {
            effect_param_mutable.value +=
                game_state.died_ennemies[&(game_state.current_turn_nb - 1)].len() as i64
                    * effect_param_mutable.sub_value_effect;
            effect_param_mutable.effect_type = EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE.to_owned();
        }

        // Process and return the new effect param
        self.process_effect_type(&effect_param_mutable, atk)
    }

    pub fn process_dodging(
        &mut self,
        atk_level: u64,
        class: &Class,
        current_dodge: u64,
        id_name: &str,
    ) {
        let dodge_info = if atk_level == ULTIMATE_LEVEL {
            DodgeInfo {
                name: id_name.to_owned(),
                is_dodging: false,
                is_blocking: false,
            }
        } else {
            let rand_nb = get_random_nb(1, 100);
            let is_dodging = *class != Class::Tank && rand_nb <= current_dodge as i64;
            let is_blocking = *class == Class::Tank;
            DodgeInfo {
                name: id_name.to_owned(),
                is_dodging,
                is_blocking,
            }
        };
        self.dodge_info = dodge_info;
    }

    pub fn remove_malus_effect(&mut self, ep: &EffectParam) -> Result<()> {
        if ep.effect_type == EFFECT_BLOCK_HEAL_ATK {
            self.is_heal_atk_blocked = false;
        }
        if ep.effect_type == EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT {
            self.update_buf(&BufTypes::DamageTx, -ep.value, true, "")?;
        }
        if ep.effect_type == EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT {
            self.update_buf(&BufTypes::DamageRx, -ep.value, true, "")?;
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_RX_BY_PERCENT {
            self.update_buf(&BufTypes::HealRx, -ep.value, true, "")?;
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_TX_BY_PERCENT {
            self.update_buf(&BufTypes::HealTx, -ep.value, true, "")?;
        }
        Ok(())
    }

    pub fn process_critical_strike(&mut self, atk: &AttackType, current_critical: i64) -> Result<bool> {
        // process passive power
        let is_crit_by_passive = self.all_buffers
            [BufTypes::NextHealAtkIsCrit as usize]
            .is_passive_enabled
            && atk.has_only_heal_effect();
        let crit_capped = 60;
        let rand_nb = get_random_nb(1, 100);
        let is_crit = rand_nb <= current_critical;

        // priority to passive
        let delta_capped = std::cmp::max(
            0,
            current_critical - crit_capped,
        );
        if is_crit && !is_crit_by_passive {
            if delta_capped > 0 {
                self.update_buf(
                    &BufTypes::DamageCritCapped,
                    delta_capped,
                    false,
                    "",
                )?;
            }
            Ok(true)
        } else if is_crit_by_passive {
            self.all_buffers[BufTypes::NextHealAtkIsCrit as usize]
                .is_passive_enabled = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        buffers::BufTypes,
        character_mod::rounds_information::{CharacterRoundsInfo, HotsBufs},
        common::{
            all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
            stats_const::HP,
        },
        players_manager::GameAtkEffects,
        testing_effect::{
            build_buf_effect_individual, build_debuf_effect_individual,
            build_dmg_effect_individual, build_dot_effect_individual, build_hot_effect_individual,
        },
    };

    #[test]
    fn unit_get_hot_and_buf_nbs() {
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&vec![]);
        assert_eq!(result, HotsBufs::default());
        let mut all_effects: Vec<GameAtkEffects> = vec![];
        // add a 1-turn-effect
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_dmg_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(result, HotsBufs::default());
        // add a 2-turn-effect HOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_hot_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 0,
                buf_nb: 0,
                debuf_nb: 0,
                hot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    30
                )],
                dot_txt: vec![],
                buf_txt: vec![],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_dot_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 0,
                debuf_nb: 0,
                hot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    30
                )],
                dot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    -20
                )],
                buf_txt: vec![],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_buf_effect_individual(),
            ..Default::default()
        });
        let result = CharacterRoundsInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 1,
                debuf_nb: 0,
                hot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    30
                )],
                dot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    -20
                )],
                buf_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::MAGICAL_ARMOR,
                    20
                )],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_debuf_effect_individual(),
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
                hot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    30
                )],
                dot_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::HP,
                    -20
                )],
                buf_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::MAGICAL_ARMOR,
                    20
                )],
                debuf_txt: vec![format!(
                    "{}-{}: {}",
                    crate::common::effect_const::EFFECT_VALUE_CHANGE,
                    crate::common::stats_const::MAGICAL_ARMOR,
                    -20
                )]
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

        // Launcher TX: BufTypes::DamageTx
        // damage buf aigainst ennemy
        cri.all_buffers
            .get_mut(BufTypes::DamageTx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, false);
        // -100 -20 = -120
        assert_eq!(result, -120);
        // same but with critical strike
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 -20 = -120 * 2 = -240
        assert_eq!(result, -240);
        cri.reset_all_buffers();

        //Receiver RX: BufTypes::DamageRx
        cri.all_buffers
            .get_mut(BufTypes::DamageRx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, false);
        // -100 -20 = -120
        assert_eq!(result, -120);
        cri.reset_all_buffers();

        //Receiver RX: BufTypes::DamageCritCapped
        cri.all_buffers
            .get_mut(BufTypes::DamageCritCapped as usize)
            .unwrap()
            .set_buffers(2, false);
        // crit is doubled init:2 -> 2 + 2 = 4
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 * 4 = -400
        assert_eq!(result, -400);

        // it can be accunulated with damage buf
        cri.all_buffers
            .get_mut(BufTypes::DamageTx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(-100, TARGET_ENNEMY, true);
        // -100 -20 = -120* 4 = -480
        assert_eq!(result, -480);
        cri.reset_all_buffers();

        // buf debuf heal against ally

        // Launcher TX: BufTypes::MultiValue
        cri.all_buffers
            .get_mut(BufTypes::MultiValue as usize)
            .unwrap()
            .set_buffers(3, false);
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 * 3 = 300
        assert_eq!(result, 300);
        cri.reset_all_buffers();

        // BufTypes::HealTx
        cri.all_buffers
            .get_mut(BufTypes::HealTx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.reset_all_buffers();
        // BufTypes::HealRx
        cri.all_buffers
            .get_mut(BufTypes::HealRx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.reset_all_buffers();
        // BufTypes::BoostedByHots
        cri.all_buffers
            .get_mut(BufTypes::BoostedByHots as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.apply_buf_debuf(100, TARGET_ALLY, false);
        // 100 + 20 = 120
        assert_eq!(result, 120);
        cri.reset_all_buffers();
    }

    #[test]
    fn unit_reset_all_buffers() {
        let mut cri = CharacterRoundsInfo::default();
        cri.all_buffers
            .get_mut(BufTypes::DamageTx as usize)
            .unwrap()
            .set_buffers(20, false);
        cri.reset_all_buffers();
        let result = cri.all_buffers.get(BufTypes::DamageTx as usize).unwrap();
        assert_eq!(result.value, 0);
        assert!(!result.is_percent);
        assert!(!result.is_passive_enabled);
    }

    #[test]
    fn unit_update_buf() {
        let mut cri = CharacterRoundsInfo::default();
        cri.all_buffers
            .get_mut(BufTypes::DamageTx as usize)
            .unwrap()
            .set_buffers(20, false);
        let result = cri.update_buf(&BufTypes::DamageTx, 10, false, HP);
        assert_eq!(30, cri.all_buffers[BufTypes::DamageTx as usize].value);
        assert!(result.is_ok());
        assert!(!cri.all_buffers[BufTypes::DamageTx as usize].is_percent);
        assert_eq!(
            HP,
            cri.all_buffers[BufTypes::DamageTx as usize].all_stats_name[0]
        );
    }
}
