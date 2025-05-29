use anyhow::{anyhow, bail, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, vec};

use crate::{
    attack_type::AttackType,
    buffers::{update_damage_by_buf, update_heal_by_multi, BufTypes, Buffers},
    common::{
        all_target_const::*,
        attak_const::{COEFF_CRIT_DMG, COEFF_CRIT_STATS},
        character_const::{NB_TURN_SUM_AGGRO, ULTIMATE_LEVEL},
        effect_const::*,
        paths_const::*,
        reach_const::*,
        stats_const::*,
    },
    effect::{
        is_boosted_by_crit, is_effect_only_at_atk_launch, process_decrease_on_turn, EffectOutcome,
        EffectParam,
    },
    equipment::Equipment,
    game_state::GameState,
    players_manager::{DodgeInfo, GameAtkEffects},
    powers::Powers,
    stats::Stats,
    stats_in_game::StatsInGame,
    target::is_target_ally,
    utils::{self, get_random_nb, list_files_in_dir},
};

/// ExtendedCharacter
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ExtendedCharacter {
    /// Fight information: Is the random character targeted by the current attack of other character
    #[serde(default, rename = "is_random_target")]
    pub is_random_target: bool,
    /// Fight information: TODO is_heal_atk_blocked
    #[serde(default, rename = "is_heal_atk_blocked")]
    pub is_heal_atk_blocked: bool,
    /// Fight information: Playing the first round of that tour
    #[serde(default, rename = "is_first_round")]
    pub is_first_round: bool,
}

impl Default for ExtendedCharacter {
    fn default() -> Self {
        ExtendedCharacter {
            is_random_target: false,
            is_heal_atk_blocked: false,
            is_first_round: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Character {
    /// Full Name of the character
    #[serde(rename = "Name")]
    pub name: String,
    /// Short name of the character
    #[serde(rename = "Short name")]
    pub short_name: String,
    /// Name of the photo of the character without extension
    #[serde(rename = "Photo")]
    pub photo_name: String,
    /// Stats about all the capacities and current state
    #[serde(rename = "Stats")]
    pub stats: Stats,
    /// Type of the character {Hero, Boss}
    #[serde(rename = "Type")]
    pub kind: CharacterType,
    /// Class of the character {Standard, Tank ...}
    #[serde(rename = "Class")]
    pub class: Class,
    /// Level of the character, start 1
    #[serde(rename = "Level")]
    pub level: u64,
    /// Experience of the character, start 0
    #[serde(rename = "Experience")]
    pub exp: u64,
    /// Experience to acquire to upgrade to next level
    pub next_exp_level: u64,
    /// key: body, value: equipmentName
    pub equipment_on: HashMap<String, Equipment>,
    /// key: attak name, value: AttakType struct
    pub attacks_list: IndexMap<String, AttackType>,
    /// That vector contains all the atks from m_AttakList and is sorted by level.
    pub attacks_by_lvl: Vec<AttackType>,
    /// Main color theme of the character
    #[serde(rename = "Color")]
    pub color_theme: String,
    /// Fight information: last attack was critical
    pub is_last_atk_crit: bool,
    /// Fight information: damages transmitted or received through the fight
    #[serde(rename = "Tx-rx")]
    pub tx_rx: Vec<HashMap<u64, i64>>,
    /// Fight information: Enabled buf/debuf acquired through the fight
    #[serde(rename = "Buf-debuf")]
    pub all_buffers: Vec<Buffers>,
    /// Powers
    #[serde(rename = "Powers")]
    pub power: Powers,
    /// ExtendedCharacter
    #[serde(rename = "ExtendedCharacter")]
    pub extended_character: ExtendedCharacter,
    /// Fight information: attack can be blocked
    #[serde(rename = "is-blocking-atk")]
    pub is_blocking_atk: bool,
    /// Fight information: nb-actions-in-round
    #[serde(rename = "nb-actions-in-round")]
    pub actions_done_in_round: u64,
    /// Fight information: max-actions-in-round
    #[serde(rename = "max-actions-by-round")]
    pub max_actions_by_round: u64,
    /// TODO rank
    #[serde(rename = "Rank")]
    pub rank: u64,
    /// TODO shape
    #[serde(rename = "Shape")]
    pub shape: String,
    #[serde(default, rename = "Effects")]
    pub all_effects: Vec<GameAtkEffects>,
    /// Fight information: dodge information on atk
    #[serde(default, rename = "dodge-info")]
    pub dodge_info: DodgeInfo,
    /// Fight information: is_current_target
    #[serde(default, rename = "is-current-target")]
    pub is_current_target: bool,
    /// Fight information: is_current_target
    #[serde(default, rename = "is-potential-target")]
    pub is_potential_target: bool,
    /// Fight information: stats_in_game
    #[serde(default)]
    stats_in_game: StatsInGame,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            name: String::from("default"),
            short_name: String::from("default"),
            photo_name: String::from("default"),
            stats: Stats::default(),
            kind: CharacterType::Hero,
            equipment_on: HashMap::new(),
            attacks_list: IndexMap::new(),
            level: 1,
            exp: 0,
            next_exp_level: 100,
            attacks_by_lvl: vec![],
            color_theme: "dark".to_owned(),
            is_last_atk_crit: false,
            all_buffers: vec![],
            is_blocking_atk: false,
            power: Powers::default(),
            extended_character: ExtendedCharacter::default(),
            actions_done_in_round: 0,
            max_actions_by_round: 0,
            class: Class::Standard,
            tx_rx: vec![HashMap::new()],
            rank: 0,
            shape: String::new(),
            all_effects: vec![],
            dodge_info: DodgeInfo::default(),
            is_current_target: false,
            is_potential_target: false,
            stats_in_game: StatsInGame::default(),
        }
    }
}

/// Defines the type of player: hero -> player, boss -> computer.
/// "PascalCase" ensures that "Hero" and "Boss" from JSON map correctly to the Rust enum variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum CharacterType {
    Hero,
    Boss,
}

/// Defines the class of the character
/// In the future, bonus and stats will be acquired.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Class {
    Standard,
    Tank,
}

impl Character {
    pub fn try_new_from_json<P1: AsRef<Path>, P2: AsRef<Path>>(
        path: P1,
        root_path: P2,
        load_from_saved_game: bool,
    ) -> Result<Character> {
        if let Ok(mut value) = utils::read_from_json::<_, Character>(&path) {
            value.stats.init();
            let txrxlen = value.tx_rx.len();
            for _ in 0..AmountType::EnumSize as usize - txrxlen {
                value.tx_rx.push(HashMap::new());
            }

            let buflen = value.all_buffers.len();
            for _ in 0..BufTypes::EnumSize as usize - buflen {
                value.all_buffers.push(Buffers::default());
            }
            // read atk only if it is new game
            if !load_from_saved_game {
                let attack_path_dir = root_path.as_ref().join(*OFFLINE_ATTACKS).join(&value.name);
                match list_files_in_dir(&attack_path_dir) {
                    Ok(list) => list.iter().for_each(|attack_path| {
                        match AttackType::try_new_from_json(attack_path) {
                            Ok(atk) => {
                                value.attacks_list.insert(atk.name.clone(), atk);
                            }
                            Err(e) => println!("{:?} cannot be decoded: {}", attack_path, e),
                        }
                    }),
                    Err(e) => bail!("Files cannot be listed in {:#?}: {}", attack_path_dir, e),
                };
            }

            Ok(value)
        } else {
            Err(anyhow!("Unknown file: {:?}", path.as_ref()))
        }
    }

    pub fn is_dead(&self) -> Option<bool> {
        if self.stats.all_stats.contains_key(HP) {
            Some(self.stats.all_stats[HP].current == 0)
        } else {
            None
        }
    }

    /**
     * @brief Character::InitAggroOnTurn
     * Set the aggro of m_LastTxRx to 0 on each turn
     * Assess the amount of aggro of the last 5 turns
     */
    pub fn init_aggro_on_turn(&mut self, turn_nb: usize) {
        if self.tx_rx.len() <= AmountType::Aggro as usize {
            return;
        }
        if let Some(aggro_stat) = self.stats.all_stats.get_mut(AGGRO) {
            aggro_stat.current = 0;
            let mut index: i64;
            for i in 1..NB_TURN_SUM_AGGRO + 1 {
                index = turn_nb as i64 - i as i64;
                if index < 0 {
                    break;
                }
                if i <= self.tx_rx[AmountType::Aggro as usize].len() {
                    let aggro = *self.tx_rx[AmountType::Aggro as usize]
                        .get(&(index as u64))
                        .unwrap_or(&0);
                    aggro_stat.current = aggro_stat.current.saturating_add(aggro as u64);
                }
            }
        }

        self.tx_rx[AmountType::Aggro as usize].insert(turn_nb as u64, 0);
    }

    /*
     * @brief Character::SetStatsOnEffect
     * @param stat
     * stat.m_RawMaxValue of a stat cannot be equal to 0.
     *
     * @param value
     * @param isPercent
     * @param updateEffect: false -> enable to update current value et max value
     * only with equipments buf.
     */
    pub fn set_stats_on_effect(
        &mut self,
        attribute_name: &str,
        value: i64,
        is_percent: bool,
        update_effect: bool,
    ) {
        let stat = self
            .stats
            .all_stats
            .get_mut(attribute_name)
            .expect("Stat not found");
        let ratio = utils::calc_ratio(stat.current as i64, stat.max as i64);
        if stat.max_raw == 0 {
            return;
        }
        let base_value =
            stat.max_raw + stat.buf_equip_value + stat.buf_equip_percent * stat.max_raw / 100;
        stat.max = base_value;
        if update_effect {
            if is_percent {
                if value > 0 {
                    stat.buf_effect_percent += value as u64;
                } else {
                    stat.buf_effect_percent = stat.buf_effect_percent.saturating_sub(value as u64);
                }
            } else if value > 0 {
                stat.buf_effect_value += value as u64;
            } else {
                stat.buf_effect_value = stat.buf_effect_percent.saturating_sub(value as u64);
            }
        }
        stat.max = base_value + stat.buf_effect_value + stat.buf_effect_percent * base_value / 100;
        stat.current = (stat.max as f64 * ratio).round() as u64;
    }

    // TODO if i remove a malus percent for DamageTx with EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT, how can if make the difference with a value which is not percent
    pub fn remove_malus_effect(&mut self, ep: &EffectParam) {
        if ep.effect_type == EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE {
            self.set_stats_on_effect(&ep.stats_name, -ep.value, true, true);
        }
        if ep.effect_type == EFFECT_IMPROVE_MAX_STAT_BY_VALUE {
            self.set_stats_on_effect(&ep.stats_name, -ep.value, false, true);
        }
        if ep.effect_type == EFFECT_BLOCK_HEAL_ATK {
            self.extended_character.is_heal_atk_blocked = false;
        }
        if ep.effect_type == EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT {
            self.update_buf(BufTypes::DamageTx, -ep.value, true, "");
        }
        if ep.effect_type == EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT {
            self.update_buf(BufTypes::DamageRx, -ep.value, true, "");
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_RX_BY_PERCENT {
            self.update_buf(BufTypes::HealRx, -ep.value, true, "");
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_TX_BY_PERCENT {
            self.update_buf(BufTypes::HealTx, -ep.value, true, "");
        }
    }

    pub fn update_buf(&mut self, buf_type: BufTypes, value: i64, is_percent: bool, stat: &str) {
        if let Some(buf) = self.all_buffers.get_mut(buf_type as usize) {
            buf.value += value;
            buf.is_percent = is_percent;
            buf.all_stats_name.push(stat.to_string());
        }
    }

    pub fn process_one_effect(
        &mut self,
        ep: &EffectParam,
        _from_launch: bool,
        atk: &AttackType,
        game_state: &GameState,
        is_crit: bool,
    ) -> (EffectParam, String) {
        let mut output = ep.clone(); // EffectParam
        let mut result: String = String::new();

        // Preprocess effectParam before applying it
        // update effectParam -> only used on in case of atk launched
        if is_crit && is_boosted_by_crit(&ep.effect_type) {
            output.sub_value_effect = (COEFF_CRIT_STATS * ep.sub_value_effect as f64) as i64;
            output.value = (COEFF_CRIT_STATS * ep.value as f64) as i64;
        }
        // conditions
        if ep.effect_type == CONDITION_ENNEMIES_DIED {
            output.value += game_state.died_ennemies[&(game_state.current_turn_nb - 1)].len()
                as i64
                * output.sub_value_effect;
            output.effect_type = EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE.to_owned();
        }

        // Process effect param
        let (effect_log, new_effect_param) = self.process_effect_type(&output, atk);
        result += &effect_log;

        (new_effect_param, result)
    }

    /// Update all the bufs
    pub fn process_effect_type(
        &mut self,
        ep: &EffectParam,
        atk: &AttackType,
    ) -> (String, EffectParam) {
        let mut output_log: String = String::new();
        let mut new_effect_param = ep.clone();
        new_effect_param.number_of_applies = 1;
        let bug_apply_init = &self.all_buffers[BufTypes::ApplyEffectInit as usize];
        if bug_apply_init.value > 0 {
            new_effect_param.number_of_applies = bug_apply_init.value;
        }

        match ep.effect_type.as_str() {
            EFFECT_NB_COOL_DOWN => {
                output_log = format!("Cooldown actif sur {} de {} tours.", atk.name, ep.nb_turns);
            }
            EFFECT_NB_DECREASE_ON_TURN => {
                // TODO
                new_effect_param.number_of_applies = process_decrease_on_turn(ep);
                self.update_buf(
                    BufTypes::ApplyEffectInit,
                    new_effect_param.number_of_applies,
                    false,
                    "",
                );
                output_log = format!(
                    "L'attaque sera effectuée {} fois.",
                    new_effect_param.number_of_applies
                );
            }
            EFFECT_REINIT => {}
            _ => {}
        }
        // Must be filled before changing value of nbTurns
        if ep.effect_type == EFFECT_REINIT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_DELETE_BAD {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_IMPROVE_HOTS {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_BOOSTED_BY_HOTS {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_RX_BY_PERCENT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_CHANGE_HEAL_TX_BY_PERCENT {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_IMPROVE_MAX_STAT_BY_VALUE {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_REPEAT_AS_MANY_AS {
            // TODO
            return (String::new(), ep.clone());
        }
        if ep.effect_type == EFFECT_INTO_DAMAGE {
            // TODO
            return (String::new(), ep.clone());
        }
        (output_log, new_effect_param)
    }

    /// Process the hot, dot and update of current value of stats
    /// Calculate the max amount of the effect
    /// Calculate the real amount transmitted to the target such as `real amount` <= `full amount`
    pub fn process_effect_value_on_new_round(
        &mut self,
        ep: &EffectParam,
        target: &mut Character,
    ) -> (i64, i64) {
        if ep.stats_name.is_empty()
            || !self.stats.all_stats.contains_key(&ep.stats_name)
            || is_effect_only_at_atk_launch(&ep.stats_name)
        {
            return (0, 0);
        }

        // calculation of the full amount of the value of the effect
        let full_amount;
        if ep.stats_name == HP && ep.effect_type == EFFECT_NB_DECREASE_ON_TURN {
            full_amount = ep.value;
        } else if ep.effect_type == EFFECT_PERCENT_CHANGE && (Stats::is_energy_stat(&ep.stats_name))
        {
            full_amount = ep.number_of_applies
                * self.stats.all_stats.get(&ep.stats_name).unwrap().max as i64
                * ep.value
                / 100;
        } else {
            full_amount = ep.number_of_applies * ep.value;
        }
        // Return now if the full amount is 0
        if full_amount == 0 {
            return (0, 0);
        }

        // Otherwise update the current value of the stats or the HOT/DOT
        // stats update
        if !Stats::is_energy_stat(&ep.stats_name) {
            target.set_stats_on_effect(
                &ep.stats_name,
                full_amount,
                ep.effect_type == EFFECT_PERCENT_CHANGE,
                true,
            );
            return (full_amount, full_amount);
        }
        // Calculation of the real amount of the value of the effect
        let real_amount = self.process_real_amount(ep, full_amount);

        (full_amount, real_amount)
    }

    pub fn apply_buf_debuf(&self, full_amount: i64, target: &str, is_crit: bool) -> i64 {
        let mut real_amount = full_amount;
        let mut buf_debuf = 0;
        let mut coeff_crit = COEFF_CRIT_DMG;
        // buf debuf heal
        if full_amount > 0 && is_target_ally(target) {
            // Launcher TX
            // To place first
            if let Some(buf_multi) = self.all_buffers.get(BufTypes::MultiValue as usize) {
                if buf_multi.value > 0 {
                    real_amount = update_heal_by_multi(full_amount, buf_multi.value);
                }
            }
            // Launcher TX
            if let Some(buf_hp_tx) = self.all_buffers.get(BufTypes::HealTx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_hp_tx.value, buf_hp_tx.is_percent, real_amount);
            }
            // Receiver RX
            if let Some(buf_hp_rx) = self.all_buffers.get(BufTypes::HealRx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_hp_rx.value, buf_hp_rx.is_percent, real_amount);
            }
            // Launcher TX
            if let Some(buf_nb_hots) = self.all_buffers.get(BufTypes::BoostedByHots as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_nb_hots.value, buf_nb_hots.is_percent, real_amount);
            }
        }
        // buf debuf damage
        if full_amount < 0 && !is_target_ally(target) {
            // Launcher TX
            if let Some(buf_dmg_tx) = self.all_buffers.get(BufTypes::DamageTx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_dmg_tx.value, buf_dmg_tx.is_percent, real_amount);
            }
            // Receiver RX
            if let Some(buf_dmg_rx) = self.all_buffers.get(BufTypes::DamageRx as usize) {
                buf_debuf +=
                    update_damage_by_buf(buf_dmg_rx.value, buf_dmg_rx.is_percent, real_amount);
            }
            // Receiver RX
            if let Some(buf_dmg_crit) = self.all_buffers.get(BufTypes::DamageCritCapped as usize) {
                // improve crit coeff
                coeff_crit += buf_dmg_crit.value as f64 / 100.0;
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

    pub fn damage_by_atk(
        target_stats: &Stats,
        launcher_stats: &Stats,
        is_magic: bool,
        atk_value: i64,
        nb_of_turns: i64,
    ) -> i64 {
        let target_armor = target_stats.get_armor_stat(is_magic);
        let launcher_pow = launcher_stats.get_power_stat(is_magic);

        let damage = atk_value - launcher_pow / nb_of_turns;
        let protection = 1000.0 / (1000.0 + target_armor as f64);

        (damage as f64 * protection).round() as i64
    }

    pub fn regen_into_damage(_real_amount_sent: i64, _stats_name: &str) -> String {
        String::new()
    }

    pub fn increment_counter_effect(&mut self) {
        for gae in self.all_effects.iter_mut() {
            gae.all_atk_effects.counter_turn += 1;
        }
    }

    pub fn remove_terminated_effect_on_player(&mut self) {
        for gae in self.all_effects.clone() {
            if gae.all_atk_effects.counter_turn == gae.all_atk_effects.nb_turns {
                // TODO add log: effect is terminated
                self.remove_malus_effect(&gae.all_atk_effects);
            }
        }
        self.all_effects.retain(|element| {
            element.all_atk_effects.nb_turns != element.all_atk_effects.counter_turn
        });
    }

    pub fn reset_all_effects_on_player(&mut self) {
        for gae in self.all_effects.clone() {
            self.remove_malus_effect(&gae.all_atk_effects);
        }
        self.all_effects.clear();
    }

    pub fn reset_all_buffers(&mut self) {
        self.all_buffers.iter_mut().for_each(|b| {
            b.set_buffers(0, false);
            b.is_passive_enabled = false;
        });
    }

    pub fn process_atk_cost(&mut self, atk_name: &str) {
        if let Some(atk) = self.attacks_list.get(atk_name) {
            if let Some(mana) = self.stats.all_stats.get_mut(MANA) {
                mana.current = std::cmp::max(0, mana.current - atk.mana_cost * mana.max / 100);
            }
            if let Some(vigor) = self.stats.all_stats.get_mut(VIGOR) {
                vigor.current = std::cmp::max(0, vigor.current - atk.vigor_cost * vigor.max / 100);
            }
            if let Some(berseck) = self.stats.all_stats.get_mut(BERSECK) {
                berseck.current =
                    std::cmp::max(0, berseck.current - atk.berseck_cost * berseck.max / 100);
            }
        }
    }

    pub fn is_dodging(&self, target: &str) -> bool {
        self.dodge_info.is_dodging && target == TARGET_ENNEMY
    }

    pub fn process_dodging(&mut self, atk_level: i64) {
        let dodge_info = if atk_level == ULTIMATE_LEVEL {
            DodgeInfo {
                name: self.name.clone(),
                is_dodging: false,
                is_blocking: false,
            }
        } else {
            let rand_nb = get_random_nb(1, 100);
            let is_dodging =
                self.class != Class::Tank && rand_nb <= self.stats.all_stats[DODGE].current as i64;
            let is_blocking = self.class == Class::Tank;
            DodgeInfo {
                name: self.name.clone(),
                is_dodging,
                is_blocking,
            }
        };
        self.dodge_info = dodge_info;
    }

    pub fn process_critical_strike(&mut self, atk_name: &str) -> bool {
        let atk = if let Some(atk) = self.attacks_list.get(atk_name) {
            atk
        } else {
            return false;
        };
        // process passive power
        let is_crit_by_passive = self.all_buffers[BufTypes::NextHealAtkIsCrit as usize]
            .is_passive_enabled
            && atk.has_only_heal_effect();
        let crit_capped = 60;
        let rand_nb = get_random_nb(1, 100);
        let is_crit = rand_nb <= self.stats.all_stats[CRITICAL_STRIKE].current as i64;

        // priority to passive
        let delta_capped = std::cmp::max(
            0,
            self.stats.all_stats[CRITICAL_STRIKE].current as i64 - crit_capped,
        );
        if is_crit && !is_crit_by_passive {
            if delta_capped > 0 {
                self.update_buf(BufTypes::DamageCritCapped, delta_capped, false, "");
            }
            true
        } else if is_crit_by_passive {
            self.all_buffers[BufTypes::NextHealAtkIsCrit as usize].is_passive_enabled = false;
            true
        } else {
            false
        }
    }

    pub fn assess_effect_param(
        &mut self,
        ep: &EffectParam,
        _from_launch: bool,
        atk: &AttackType,
        game_state: &GameState,
        is_crit: bool,
    ) -> EffectParam {
        let (ec, _log) = self.process_one_effect(ep, true, atk, game_state, is_crit);

        ec
    }

    pub fn is_targeted(
        &self,
        effect: &EffectParam,
        launcher_name: &str,
        launcher_kind: &CharacterType,
    ) -> bool {
        let is_ally = self.kind == *launcher_kind;
        if effect.target == TARGET_HIMSELF && launcher_name != self.name {
            return false;
        }
        if effect.target == TARGET_ONLY_ALLY && launcher_name == self.name {
            return false;
        }
        if !is_ally && is_target_ally(&effect.target) {
            return false;
        }
        if is_ally && effect.target == TARGET_ENNEMY {
            return false;
        }
        // is targeted ?
        if effect.target == TARGET_ALLY && effect.reach == INDIVIDUAL && !self.is_current_target {
            return false;
        }
        if effect.target == TARGET_ENNEMY && effect.reach == INDIVIDUAL && !self.is_current_target {
            return false;
        }
        if effect.target == TARGET_ALLY && effect.reach == ZONE && launcher_name == self.name {
            return false;
        }
        if self.is_dodging(&effect.target) {
            return false;
        }
        // TODO reach random
        /* if effect.reach == REACH_RAND_INDIVIDUAL && target.m_ext_character.is_some()
            && !target.m_ext_character.as_ref().unwrap().get_is_random_target() {
                return false;
        } */
        true
    }

    pub fn apply_effect_outcome(
        &mut self,
        ep: &EffectParam,
        launcher_stats: &Stats,
        is_crit: bool,
        current_turn: usize, // to process aggro
    ) -> EffectOutcome {
        if ep.stats_name.is_empty() || !self.stats.all_stats.contains_key(&ep.stats_name) {
            return EffectOutcome::default();
        }
        let mut full_amount;
        let mut new_effect_param = ep.clone();
        let pow_current = launcher_stats.get_power_stat(ep.is_magic_atk);
        if ep.stats_name == HP && ep.effect_type == EFFECT_NB_DECREASE_ON_TURN {
            // prepare for HOT
            full_amount = ep.number_of_applies * (ep.value + pow_current / ep.nb_turns);
            // update effect value
            new_effect_param.value = full_amount;
        } else if ep.stats_name == HP && ep.effect_type == EFFECT_VALUE_CHANGE
            || ep.effect_type == EFFECT_PERCENT_CHANGE
        {
            if ep.value > 0 {
                // HOT
                full_amount = ep.number_of_applies * (ep.value + pow_current) / ep.nb_turns;
            } else {
                // DOT
                full_amount = ep.number_of_applies
                    * Self::damage_by_atk(
                        &self.stats,
                        launcher_stats,
                        ep.is_magic_atk,
                        ep.value,
                        ep.nb_turns,
                    );
            }
        } else if ep.effect_type == EFFECT_PERCENT_CHANGE && Stats::is_energy_stat(&ep.stats_name) {
            full_amount = ep.number_of_applies
                * self.stats.all_stats.get(&ep.stats_name).unwrap().max as i64
                * ep.value
                / 100;
        } else {
            full_amount = ep.number_of_applies * ep.value;
        }
        // Return now if the full amount is 0
        if full_amount == 0 {
            return EffectOutcome::default();
        }

        // apply buf/debuf to full_amount in case of damages/heal
        if ep.stats_name == HP {
            full_amount = self.apply_buf_debuf(full_amount, &ep.target, is_crit);
            new_effect_param.value = full_amount;
        }

        // Otherwise update the current value of the stats or the HOT/DOT
        // stats update
        if !Stats::is_energy_stat(&ep.stats_name) {
            self.set_stats_on_effect(
                &ep.stats_name,
                full_amount,
                ep.effect_type == EFFECT_PERCENT_CHANGE,
                true,
            );
            return EffectOutcome {
                full_atk_amount_tx: full_amount,
                real_amount_tx: full_amount,
                new_effect_param,
                target_name: self.name.clone(),
                ..Default::default()
            };
        }

        // blocking the atk
        if self.dodge_info.is_blocking && ep.stats_name == HP && ep.target == TARGET_ENNEMY {
            full_amount = 10 * full_amount / 100;
        }
        // Calculation of the real amount of the value of the effect and update the energy stats
        let real_amount = self.process_real_amount(ep, full_amount);

        // process aggro
        if ep.effect_type != EFFECT_IMPROVE_MAX_STAT_BY_VALUE
            && ep.effect_type != EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE
        {
            if ep.stats_name == HP {
                // process aggro for the launcher
                self.process_aggro(real_amount, 0, current_turn);
            } else {
                // Add aggro to a target
                self.process_aggro(0, ep.value, current_turn);
            }
        }

        // update stats in game
        let eo = EffectOutcome {
            full_atk_amount_tx: full_amount,
            real_amount_tx: real_amount,
            new_effect_param,
            target_name: self.name.clone(),
            ..Default::default()
        };
        self.stats_in_game.update_by_effectoutcome(&eo);

        eo
    }

    pub fn process_atk(
        &mut self,
        game_state: &GameState,
        is_crit: bool,
        atk: &AttackType,
    ) -> Vec<EffectParam> {
        let mut output: Vec<EffectParam> = vec![];
        for effect in atk.all_effects.clone() {
            output.push(self.assess_effect_param(&effect, true, atk, game_state, is_crit));
        }
        output
    }

    /// access the real amount received by the effect on that character
    pub fn process_real_amount(&mut self, ep: &EffectParam, full_amount: i64) -> i64 {
        if ep.stats_name != HP {
            return 0;
        }
        let real_amount;
        if full_amount > 0 {
            // heal
            let delta =
                self.stats.all_stats[HP].max as i64 - self.stats.all_stats[HP].current as i64;
            self.stats.all_stats[HP].current = std::cmp::min(
                full_amount + self.stats.all_stats[HP].current as i64,
                self.stats.all_stats[HP].max as i64,
            ) as u64;
            real_amount = std::cmp::min(delta, full_amount);
        } else {
            // damage
            let tmp = self.stats.all_stats[HP].current as i64;
            self.stats.all_stats[HP].current =
                std::cmp::max(0, self.stats.all_stats[HP].current as i64 + full_amount) as u64;
            real_amount = std::cmp::max(-tmp, full_amount);
        }
        real_amount
    }

    pub fn process_aggro(&mut self, atk_value: i64, aggro_value: i64, turn_nb: usize) {
        let aggro_norm = 20.0;
        let mut local_aggro = aggro_value as f64;
        // Aggro filled by atkValue or input aggro value ?
        if atk_value != 0 {
            local_aggro = (atk_value.abs() as f64 / aggro_norm).round();
        }
        // case null aggro
        if local_aggro == 0.0 {
            return;
        }
        // Update aggro
        if let Some(aggro_stat) = self.stats.all_stats.get_mut(AGGRO) {
            if let Some(tx_map) = self.tx_rx.get_mut(AmountType::Aggro as usize) {
                if let Some(aggro) = tx_map.get_mut(&(turn_nb as u64)) {
                    // update txrx current turn nb
                    *aggro += local_aggro as i64;
                    // update stats aggro of character
                    aggro_stat.current += *aggro as u64;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::Character;
    use crate::character::AmountType;
    use crate::effect::EffectOutcome;
    use crate::testing_all_characters::testing_character;
    use crate::{
        buffers::BufTypes,
        character::{CharacterType, Class},
        common::{all_target_const::TARGET_ALLY, effect_const::*, stats_const::*},
        effect::EffectParam,
        players_manager::GameAtkEffects,
        testing_effect::*,
    };

    #[test]
    fn unit_try_new_from_json() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let root_path = "./tests/offlines";
        let c = Character::try_new_from_json(file_path, root_path, false);
        assert!(c.is_ok());
        let c = c.unwrap();
        // name
        assert_eq!("test", c.name);
        assert_eq!("test", c.short_name);
        // buf-debuf
        assert_eq!(12, c.all_buffers.len());
        // TODO change
        //assert_eq!("hp,mana", c.all_buffers[0].all_stats_name);
        assert_eq!(3, c.all_buffers[0].buf_type);
        assert_eq!(false, c.all_buffers[0].is_passive_enabled);
        assert_eq!(true, c.all_buffers[0].is_percent);
        assert_eq!(100, c.all_buffers[0].value);
        // Class
        assert_eq!(Class::Standard, c.class);
        // Color
        assert_eq!("green", c.color_theme);
        // Experience
        assert_eq!(50, c.exp);
        // extended character
        assert_eq!(true, c.extended_character.is_first_round);
        assert_eq!(true, c.extended_character.is_heal_atk_blocked);
        assert_eq!(false, c.extended_character.is_random_target);
        // level
        assert_eq!(1, c.level);
        // photo
        assert_eq!("phototest", c.photo_name);
        // powers
        assert_eq!(false, c.power.is_crit_heal_after_crit);
        assert_eq!(true, c.power.is_damage_tx_heal_needy_ally);
        // rank
        assert_eq!(4, c.rank);
        // shape
        assert_eq!("", c.shape);
        // stats
        // stats - aggro
        assert_eq!(0, c.stats.all_stats[AGGRO].current);
        assert_eq!(9999, c.stats.all_stats[AGGRO].max);
        // stats - aggro rate
        assert_eq!(1, c.stats.all_stats[AGGRO_RATE].current);
        assert_eq!(1, c.stats.all_stats[AGGRO_RATE].max);
        // stats - berseck
        assert_eq!(200, c.stats.all_stats[BERSECK].current);
        assert_eq!(200, c.stats.all_stats[BERSECK].max);
        // stats - berseck_rate
        assert_eq!(1, c.stats.all_stats[BERSECK_RATE].current);
        assert_eq!(1, c.stats.all_stats[BERSECK_RATE].max);
        // stats - critical_strike
        assert_eq!(10, c.stats.all_stats[CRITICAL_STRIKE].current);
        assert_eq!(10, c.stats.all_stats[CRITICAL_STRIKE].max);
        // stats - dodge
        assert_eq!(5, c.stats.all_stats[DODGE].current);
        assert_eq!(5, c.stats.all_stats[DODGE].max);
        // stats - hp
        assert_eq!(1, c.stats.all_stats[HP].current);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(135, c.stats.all_stats[HP].max_raw);
        assert_eq!(1, c.stats.all_stats[HP].current_raw);
        // stats - hp_regeneration
        assert_eq!(7, c.stats.all_stats[HP_REGEN].current);
        assert_eq!(7, c.stats.all_stats[HP_REGEN].max);
        // stats - magic_armor
        assert_eq!(10, c.stats.all_stats[MAGICAL_ARMOR].current);
        assert_eq!(10, c.stats.all_stats[MAGICAL_ARMOR].max);
        // stats - magic_power
        assert_eq!(20, c.stats.all_stats[MAGICAL_POWER].current);
        assert_eq!(20, c.stats.all_stats[MAGICAL_POWER].max);
        // stats - mana
        assert_eq!(200, c.stats.all_stats[MANA].current);
        assert_eq!(200, c.stats.all_stats[MANA].max);
        // stats - mana_regeneration
        assert_eq!(7, c.stats.all_stats[MANA_REGEN].current);
        assert_eq!(7, c.stats.all_stats[MANA_REGEN].max);
        // stats - physical_armor
        assert_eq!(5, c.stats.all_stats[PHYSICAL_ARMOR].current);
        assert_eq!(5, c.stats.all_stats[PHYSICAL_ARMOR].max);
        // stats - physical_power
        assert_eq!(10, c.stats.all_stats[PHYSICAL_POWER].current);
        assert_eq!(10, c.stats.all_stats[PHYSICAL_POWER].max);
        // stats - speed
        assert_eq!(212, c.stats.all_stats[SPEED].current);
        assert_eq!(212, c.stats.all_stats[SPEED].max);
        // stats - speed_regeneration
        assert_eq!(12, c.stats.all_stats[SPEED_REGEN].current);
        assert_eq!(12, c.stats.all_stats[SPEED_REGEN].max);
        // stats - vigor
        assert_eq!(200, c.stats.all_stats[VIGOR].current);
        assert_eq!(200, c.stats.all_stats[VIGOR].max);
        // stats - vigor_regeneration
        assert_eq!(5, c.stats.all_stats[VIGOR_REGEN].current);
        assert_eq!(5, c.stats.all_stats[VIGOR_REGEN].max);
        // tx-rx
        assert_eq!(7, c.tx_rx.len());
        /*         assert_eq!(0, c.tx_rx[2].tx_rx_size);
        assert_eq!(2, c.tx_rx[2].tx_rx_type); */
        // Type - kind
        assert_eq!(CharacterType::Hero, c.kind);
        // is-blocking-atk
        assert_eq!(false, c.is_blocking_atk);
        // max_actions_by_round
        assert_eq!(1, c.max_actions_by_round);
        // nb-actions-in-round
        assert_eq!(0, c.actions_done_in_round);
        // atk
        assert_eq!(6, c.attacks_list.len());

        let file_path = "./tests/offlines/characters/wrong.json";
        let root_path = "./tests/offlines";
        assert!(Character::try_new_from_json(file_path, root_path, false).is_err());
    }

    #[test]
    fn unit_is_dead() {
        let mut c = Character::default();
        c.stats.init();
        assert!(c.is_dead().is_some());
        assert_eq!(true, c.is_dead().unwrap());
        c.stats.all_stats.get_mut(HP).unwrap().current = 15;
        assert_eq!(false, c.is_dead().unwrap());
    }

    #[test]
    fn unit_init_aggro_on_turn() {
        let mut c = Character::default();
        c.stats.init();
        c.init_aggro_on_turn(1);
        assert_eq!(0, c.stats.all_stats[AGGRO].current);
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx.push(HashMap::new());
        c.tx_rx[5].insert(1, 10);
        c.init_aggro_on_turn(2);
        assert_eq!(10, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(2, 20);
        c.init_aggro_on_turn(3);
        assert_eq!(30, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(3, 30);
        c.init_aggro_on_turn(4);
        assert_eq!(60, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(4, 40);
        c.init_aggro_on_turn(5);
        assert_eq!(100, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(5, 50);
        c.init_aggro_on_turn(6);
        assert_eq!(150, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(6, 60);
        c.init_aggro_on_turn(7);
        assert_eq!(200, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(7, 70);
        c.init_aggro_on_turn(8);
        assert_eq!(250, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(8, 80);
        c.init_aggro_on_turn(9);
        assert_eq!(300, c.stats.all_stats[AGGRO].current);
        c.tx_rx[5].insert(9, 90);
        c.init_aggro_on_turn(10);
        assert_eq!(350, c.stats.all_stats[AGGRO].current);
    }

    #[test]
    fn unit_set_stats_on_effect() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let root_path = "./tests/offlines";
        let c = Character::try_new_from_json(file_path, root_path, false);
        assert!(c.is_ok());
        let mut c = c.unwrap();
        c.set_stats_on_effect(HP, 10, false, true);
        assert_eq!(145, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.set_stats_on_effect(HP, -10, false, true);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.set_stats_on_effect(HP, 10, true, true);
        assert_eq!(148, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        c.set_stats_on_effect(HP, -10, true, true);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
    }

    #[test]
    fn unitremove_malus_effect() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let root_path = "./tests/offlines";
        let c = Character::try_new_from_json(file_path, root_path, false);
        assert!(c.is_ok());
        let mut c = c.unwrap();
        let ep = EffectParam {
            effect_type: EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        let ep = EffectParam {
            effect_type: EFFECT_IMPROVE_MAX_STAT_BY_VALUE.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        let ep = EffectParam {
            effect_type: EFFECT_BLOCK_HEAL_ATK.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(false, c.extended_character.is_heal_atk_blocked);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(-10, c.all_buffers[BufTypes::DamageTx as usize].value);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(-10, c.all_buffers[BufTypes::DamageRx as usize].value);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_HEAL_RX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(-10, c.all_buffers[BufTypes::HealRx as usize].value);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_HEAL_TX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        c.remove_malus_effect(&ep);
        assert_eq!(-10, c.all_buffers[BufTypes::HealTx as usize].value);
    }

    #[test]
    fn unit_update_buf() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let root_path = "./tests/offlines";
        let c = Character::try_new_from_json(file_path, root_path, false);
        assert!(c.is_ok());
        let mut c = c.unwrap();
        c.update_buf(BufTypes::DamageTx, 10, false, HP);
        assert_eq!(10, c.all_buffers[BufTypes::DamageTx as usize].value);
        assert_eq!(false, c.all_buffers[BufTypes::DamageTx as usize].is_percent);
        assert_eq!(
            HP,
            c.all_buffers[BufTypes::DamageTx as usize].all_stats_name[0]
        );
    }

    #[test]
    fn unit_process_one_effect() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let root_path = "./tests/offlines";
        let c = Character::try_new_from_json(file_path, root_path, false);
        assert!(c.is_ok());
        let mut c = c.unwrap();
        let ep = EffectParam {
            effect_type: EFFECT_NB_COOL_DOWN.to_string(),
            nb_turns: 10,
            target: c.name.clone(),
            ..Default::default()
        };
        let atk = Default::default();
        let game_state = Default::default();
        // target is himself
        let (ep, result) = c.process_one_effect(&ep, false, &atk, &game_state, false);
        assert_eq!(EFFECT_NB_COOL_DOWN, ep.effect_type);
        assert_eq!(10, ep.nb_turns);
        assert_eq!(c.name, ep.target);
        assert_eq!("Cooldown actif sur  de 10 tours.", result);
    }

    #[test]
    fn unit_remove_terminated_effect_on_player() {
        let mut c = testing_character();
        c.all_effects.push(GameAtkEffects::default());
        c.remove_terminated_effect_on_player();
        assert_eq!(0, c.all_effects.len());
        // TODO improve the test  by checking if the effect is removed on character stats
    }

    #[test]
    fn unit_process_atk_cost() {
        let mut c = testing_character();
        let old_mana = c.stats.all_stats[MANA].current;
        c.process_atk_cost("atk1"); // 10% mana cost
        assert_eq!(old_mana - 20, c.stats.all_stats[MANA].current);
        c.process_atk_cost("atk1"); // 10% mana cost again!
        assert_eq!(old_mana - 40, c.stats.all_stats[MANA].current);
    }

    #[test]
    fn unit_process_dodging() {
        let mut c = testing_character();

        // ultimate atk cannot be dodged
        let atk_level = 13;
        c.process_dodging(atk_level);
        assert_eq!(false, c.dodge_info.is_dodging);
        assert_eq!(false, c.dodge_info.is_blocking);

        // impossible to dodge
        let atk_level = 1;
        c.stats.all_stats[DODGE].current = 0;
        c.process_dodging(atk_level);
        assert_eq!(false, c.dodge_info.is_dodging);
        assert_eq!(false, c.dodge_info.is_blocking);

        // total dodge
        let atk_level = 1;
        c.stats.all_stats[DODGE].current = 100;
        c.process_dodging(atk_level);
        assert_eq!(true, c.dodge_info.is_dodging);
        assert_eq!(false, c.dodge_info.is_blocking);

        // A tank is not dodging, he is blocking
        let atk_level = 1;
        c.stats.all_stats[DODGE].current = 100;
        c.class = Class::Tank;
        c.process_dodging(atk_level);
        assert_eq!(false, c.dodge_info.is_dodging);
        assert_eq!(true, c.dodge_info.is_blocking);
    }

    #[test]
    fn unit_process_critical_strike() {
        let mut c = testing_character();
        c.stats.all_stats[CRITICAL_STRIKE].current = 0;
        assert_eq!(false, c.process_critical_strike("atk1"));
        c.stats.all_stats[CRITICAL_STRIKE].current = 100;
        assert_eq!(true, c.process_critical_strike("atk1"));
        assert_eq!(
            false,
            c.all_buffers[BufTypes::NextHealAtkIsCrit as usize].is_passive_enabled
        );
    }

    #[test]
    fn unit_assess_effect_param() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let root_path = "./tests/offlines";
        let c = Character::try_new_from_json(file_path, root_path, false);
        assert!(c.is_ok());
        let mut c = c.unwrap();
        let ep = EffectParam {
            effect_type: EFFECT_NB_COOL_DOWN.to_string(),
            nb_turns: 10,
            target: c.name.clone(),
            ..Default::default()
        };
        let atk = Default::default();
        let game_state = Default::default();
        // target is himself
        let ep = c.assess_effect_param(&ep, false, &atk, &game_state, false);
        assert_eq!(EFFECT_NB_COOL_DOWN, ep.effect_type);
        assert_eq!(10, ep.nb_turns);
        assert_eq!(c.name, ep.target);
    }

    #[test]
    fn unit_is_targeted() {
        let root_path = "./tests/offlines";
        let c1 =
            Character::try_new_from_json("./tests/offlines/characters/test.json", root_path, false)
                .unwrap();
        let mut c2 =
            Character::try_new_from_json("./tests/offlines/characters/test.json", root_path, false)
                .unwrap();
        c2.name = "other".to_string();
        let mut boss1 = Character::try_new_from_json(
            "./tests/offlines/characters/test_boss.json",
            root_path,
            false,
        )
        .unwrap();
        // effect on himself
        let mut ep = build_cooldown_effect();
        // target is himself
        assert_eq!(true, c1.is_targeted(&ep, &c1.name, &c1.kind));
        // other ally
        assert_eq!(false, c2.is_targeted(&ep, &c1.name, &c1.kind));
        // boss
        assert_eq!(false, boss1.is_targeted(&ep, &c1.name, &c1.kind));

        // effect on ally individual
        ep = build_hot_effect_individual();
        // target is himself
        assert_eq!(false, c1.is_targeted(&ep, &c1.name, &c1.kind));
        // other ally
        // not targeted on main atk
        c2.is_current_target = false;
        assert_eq!(false, c2.is_targeted(&ep, &c1.name, &c1.kind));
        // targeted on main atk
        c2.is_current_target = true;
        assert_eq!(true, c2.is_targeted(&ep, &c1.name, &c1.kind));
        // boss
        assert_eq!(false, boss1.is_targeted(&ep, &c1.name, &c1.kind));

        // effect on ennemy individual
        ep = build_dmg_effect_individual();
        assert_eq!(false, c1.is_targeted(&ep, &c1.name, &c1.kind));
        // other ally
        assert_eq!(false, c2.is_targeted(&ep, &c1.name, &c1.kind));
        // boss
        // targeted on main atk
        boss1.is_current_target = true;
        assert_eq!(true, boss1.is_targeted(&ep, &c1.name, &c1.kind));
        // not targeted on main atk
        boss1.is_current_target = false;
        assert_eq!(false, boss1.is_targeted(&ep, &c1.name, &c1.kind));

        // effect on ally ZONE
        ep = build_hot_effect_zone();
        // target is himself
        assert_eq!(false, c1.is_targeted(&ep, &c1.name, &c1.kind));
        // other ally
        // targeted on main atk
        assert_eq!(true, c2.is_targeted(&ep, &c1.name, &c1.kind));
        // boss
        assert_eq!(false, boss1.is_targeted(&ep, &c1.name, &c1.kind));

        // effect on ennemy ZONE
        ep = build_dot_effect_zone();
        // target is himself
        assert_eq!(false, c1.is_targeted(&ep, &c1.name, &c1.kind));
        // other ally
        assert_eq!(false, c2.is_targeted(&ep, &c1.name, &c1.kind));
        // boss
        // targeted on main atk
        boss1.is_current_target = true;
        assert_eq!(true, boss1.is_targeted(&ep, &c1.name, &c1.kind));
        // not targeted on main atk
        boss1.is_current_target = false;
        assert_eq!(true, boss1.is_targeted(&ep, &c1.name, &c1.kind));

        // effect on all allies
        ep = build_hot_effect_all();
        // target is himself
        assert_eq!(true, c1.is_targeted(&ep, &c1.name, &c1.kind));
        assert_eq!(true, c1.is_targeted(&ep, &c1.name, &c1.kind));
        // other ally
        assert_eq!(true, c2.is_targeted(&ep, &c1.name, &c1.kind));
        assert_eq!(true, c2.is_targeted(&ep, &c1.name, &c1.kind));
        // boss
        // targeted on main atk
        boss1.is_current_target = true;
        assert_eq!(false, boss1.is_targeted(&ep, &c1.name, &c1.kind));
        boss1.is_current_target = false;
        assert_eq!(false, boss1.is_targeted(&ep, &c1.name, &c1.kind));
    }

    #[test]
    fn unit_apply_effect_outcome() {
        let root_path = "./tests/offlines";
        let mut c =
            Character::try_new_from_json("./tests/offlines/characters/test.json", root_path, false)
                .unwrap();
        let mut c2 =
            Character::try_new_from_json("./tests/offlines/characters/test.json", root_path, false)
                .unwrap();
        let mut ep = build_cooldown_effect();
        let launcher_stats = c.stats.clone();
        // target is himself
        let eo = c.apply_effect_outcome(&ep, &launcher_stats, false, 0);
        assert_eq!(eo, EffectOutcome::default());

        // target is other ally
        ep = build_hot_effect_individual();
        let old_hp = c2.stats.all_stats[HP].current;
        let eo = c2.apply_effect_outcome(&ep, &launcher_stats, false, 0);
        assert_eq!(eo.full_atk_amount_tx, 20);
        assert_eq!(eo.real_amount_tx, 20);
        assert_eq!(eo.new_effect_param.value, 20);
        assert_eq!(old_hp + 20, c2.stats.all_stats[HP].current);
        assert_eq!(eo.new_effect_param.effect_type, EFFECT_VALUE_CHANGE);
        assert_eq!(eo.new_effect_param.stats_name, HP);
        assert_eq!(eo.new_effect_param.nb_turns, 2);
        assert_eq!(eo.new_effect_param.number_of_applies, 1);
        assert_eq!(eo.new_effect_param.is_magic_atk, false);
        assert_eq!(eo.new_effect_param.target, TARGET_ALLY);

        // target is ennemy
        let mut boss1 = Character::try_new_from_json(
            "./tests/offlines/characters/test_boss.json",
            root_path,
            false,
        )
        .unwrap();
        ep = build_dmg_effect_individual();
        let old_hp = boss1.stats.all_stats[HP].current;
        let eo = boss1.apply_effect_outcome(&ep, &launcher_stats, false, 0);
        assert_eq!(eo.full_atk_amount_tx, -40);
        assert_eq!(eo.real_amount_tx, -40);
        assert_eq!(eo.new_effect_param.value, -40);
        assert_eq!(old_hp - 40, boss1.stats.all_stats[HP].current);
    }

    #[test]
    fn unit_process_real_amount() {
        let root_path = "./tests/offlines";
        let mut c =
            Character::try_new_from_json("./tests/offlines/characters/test.json", root_path, false)
                .unwrap();
        let old_hp = c.stats.all_stats[HP].current;
        let result = c.process_real_amount(
            &build_dmg_effect_individual(),
            -(c.stats.all_stats[HP].current as i64) - 10,
        );
        // real amount cannot excess the life of the character
        assert_eq!(result, -(old_hp as i64));
    }

    #[test]
    fn unit_proces_aggro() {
        let root_path = "./tests/offlines";
        let mut c =
            Character::try_new_from_json("./tests/offlines/characters/test.json", root_path, false)
                .unwrap();
        c.init_aggro_on_turn(0);
        c.process_aggro(0, 0, 0);
        assert_eq!(0, c.tx_rx[AmountType::Aggro as usize][&0]);

        c.process_aggro(20, 0, 0);
        assert_eq!(1, c.tx_rx[AmountType::Aggro as usize][&0]);
    }
}
