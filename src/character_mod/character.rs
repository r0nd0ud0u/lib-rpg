use anyhow::{Result, anyhow, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, vec};

use crate::{
    character_mod::{
        attack_type::{AttackType, LauncherAtkInfo},
        buffers::BufKinds,
        class::Class,
        effect::{EffectOutcome, EffectParam, ProcessedEffectParam},
        energy::{Energy, EnergyKind},
        equipment::{Equipment, EquipmentJsonKey},
        experience::build_exp_to_next_level,
        inventory::{Consumable, Inventory},
        rank::Rank,
        rounds_information::{AmountType, CharacterRoundsInfo},
        stats::Stats,
        target::TargetData,
    },
    common::{
        constants::{all_target_const::*, paths_const::*, stats_const::*},
        log_data::{
            LogData,
            const_colors::{DARK_RED, LIGHT_GREEN},
        },
    },
    server::{
        game_state::GameState,
        players_manager::{DodgeInfo, GameAtkEffect},
    },
    utils::{self, list_files_in_dir},
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Character {
    /// Full Name of the character to query the datamanager and display in the game
    #[serde(rename = "Name")]
    pub db_full_name: String,
    /// Short name of the character
    #[serde(rename = "Short name")]
    pub short_name: String,
    /// In case there is a need to identify several characters with same name during a game
    #[serde(rename = "IdName")]
    pub id_name: String,
    /// Name of the photo of the character without extension
    #[serde(rename = "Photo")]
    pub photo_name: String,
    /// Stats about all the capacities and current state
    #[serde(rename = "Stats")]
    pub stats: Stats,
    /// Type of the character {Hero, Boss}
    #[serde(rename = "Type")]
    pub kind: CharacterKind,
    /// Class of the character {Standard, Tank ...}
    #[serde(rename = "Class")]
    pub class: Class,
    /// Level of the character, start 1
    #[serde(rename = "Level")]
    pub level: u64,
    /// key: attak name, value: AttakType struct
    pub attacks_list: IndexMap<String, AttackType>,
    /// Main color theme of the character
    #[serde(rename = "Color")]
    pub color_theme: String,
    /// CharacterRoundsInfo
    #[serde(rename = "CharacterRoundsInfo")]
    pub character_rounds_info: CharacterRoundsInfo,
    /// Inventory
    pub inventory: Inventory,
    /// Energy
    pub energies: Vec<Energy>,
    /// Rank of the character, used for boss to adapt the difficulty of the fight
    #[serde(rename = "Rank")]
    pub rank: Rank,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            db_full_name: String::from("default"),
            short_name: String::from("default"),
            id_name: String::from("_#1"),
            photo_name: String::from("default"),
            stats: Stats::default(),
            kind: CharacterKind::Hero,
            attacks_list: IndexMap::new(),
            level: 1,
            color_theme: "dark".to_owned(),
            character_rounds_info: CharacterRoundsInfo::default(),
            class: Class::Standard,
            inventory: Inventory::default(),
            energies: Vec::new(),
            rank: Rank::default(),
        }
    }
}

/// Defines the type of player: hero -> player, boss -> computer.
/// "PascalCase" ensures that "Hero" and "Boss" from JSON map correctly to the Rust enum variants.
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum CharacterKind {
    #[default]
    Hero,
    Boss,
}

impl Character {
    pub fn try_new_from_json<P1: AsRef<Path>, P2: AsRef<Path>>(
        path: P1,
        root_path: P2,
        load_from_saved_game: bool,
        all_equipments: &HashMap<EquipmentJsonKey, Vec<Equipment>>,
    ) -> Result<Character> {
        if let Ok(mut value) = utils::read_from_json::<_, Character>(&path) {
            // init stats
            value.stats.init();
            // init exp_to_next_level based on hero's rank, class and current level
            value.character_rounds_info.exp_to_next_level =
                build_exp_to_next_level(&value.rank, &value.class, value.level);
            // init tx rx table
            let txrxlen = value.character_rounds_info.tx_rx.len();
            for _ in 0..AmountType::EnumSize as usize - txrxlen {
                value.character_rounds_info.tx_rx.push(HashMap::new());
            }
            // read atk only if it is new game
            if !load_from_saved_game {
                // attack loading
                let attack_path_dir = root_path
                    .as_ref()
                    .join(*OFFLINE_ATTACKS)
                    .join(&value.db_full_name);
                match list_files_in_dir(&attack_path_dir) {
                    Ok(list) => list.iter().for_each(|attack_path| {
                        match AttackType::try_new_from_json(attack_path) {
                            Ok(atk) => {
                                value.attacks_list.insert(atk.name.clone(), atk);
                            }
                            Err(e) => tracing::error!("{:?} cannot be decoded: {}", attack_path, e),
                        }
                    }),
                    Err(e) => bail!("Files cannot be listed in {:#?}: {}", attack_path_dir, e),
                };
                let equipment_on: HashMap<EquipmentJsonKey, Vec<Equipment>> =
                    value.inventory.get_all_equipments(
                        all_equipments
                            .values()
                            .flatten()
                            .cloned()
                            .collect::<Vec<Equipment>>()
                            .as_slice(),
                        true,
                    );
                // apply equipment on stats
                value.stats.apply_equipment_on_stats(
                    &equipment_on
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<Equipment>>(),
                );
                // apply buf debuf on stats
                value
                    .stats
                    .apply_buf_debuf_on_stats(&value.character_rounds_info.all_buffers);
            }

            Ok(value)
        } else {
            Err(anyhow!("Unknown file: {:?}", path.as_ref()))
        }
    }

    /// Set the aggro of m_LastTxRx to 0 on each turn
    /// Assess the amount of aggro of the last 5 turns
    pub fn init_aggro_on_turn(&mut self, turn_nb: usize) {
        if self.character_rounds_info.tx_rx.len() <= AmountType::Aggro as usize {
            return;
        }
        self.stats.init_aggro_on_turn(
            turn_nb,
            &self.character_rounds_info.tx_rx[AmountType::Aggro as usize],
        );

        self.character_rounds_info.tx_rx[AmountType::Aggro as usize].insert(turn_nb as u64, 0);
    }

    pub fn remove_malus_effect(&mut self, ep: &EffectParam) -> Result<()> {
        if ep.buffer.kind == BufKinds::ChangeMaxStatByPercentage
            || ep.buffer.kind == BufKinds::ChangeMaxStatByValue
        {
            self.stats.set_stats_on_effect(
                &ep.buffer.stats_name,
                -ep.buffer.value,
                ep.buffer.kind == BufKinds::ChangeMaxStatByPercentage,
                true,
            );
        }
        self.character_rounds_info.remove_malus_effect(ep)?;
        Ok(())
    }

    pub fn remove_terminated_effect_on_player(&mut self) -> Result<Vec<EffectParam>> {
        let mut ended_effects: Vec<EffectParam> = Vec::new();
        for gae in self.character_rounds_info.all_effects.clone() {
            if gae.processed_effect_param.counter_turn
                == gae.processed_effect_param.input_effect_param.nb_turns
            {
                self.remove_malus_effect(&gae.processed_effect_param.input_effect_param)?;
                ended_effects.push(gae.processed_effect_param.input_effect_param.clone());
            }
        }
        self.character_rounds_info.all_effects.retain(|element| {
            element.processed_effect_param.input_effect_param.nb_turns
                != element.processed_effect_param.counter_turn
        });
        Ok(ended_effects)
    }

    pub fn reset_all_effects_on_player(&mut self) -> Result<()> {
        for gae in self.character_rounds_info.all_effects.clone() {
            self.remove_malus_effect(&gae.processed_effect_param.input_effect_param)?;
        }
        self.character_rounds_info.all_effects.clear();
        Ok(())
    }

    pub fn process_atk_cost(&mut self, atk_name: &str) {
        if let Some(atk) = self.attacks_list.get(atk_name) {
            self.stats.apply_cost_on_stats(atk.mana_cost, MANA);
            self.stats.apply_cost_on_stats(atk.berseck_cost, BERSERK);
            self.stats.apply_cost_on_stats(atk.vigor_cost, VIGOR);
        }
    }

    pub fn process_dodging(&mut self, atk_level: u64) {
        self.character_rounds_info.process_dodging(
            atk_level,
            &self.class,
            self.stats.all_stats[DODGE].current,
            &self.id_name,
        );
    }

    pub fn process_critical_strike(&mut self, atk_name: &str) -> Result<bool> {
        let atk = if let Some(atk) = self.attacks_list.get(atk_name) {
            atk
        } else {
            return Ok(false);
        };

        self.character_rounds_info
            .process_critical_strike(atk, self.stats.all_stats[CRITICAL_STRIKE].current as i64)
    }

    pub fn apply_processed_effect_param(
        &mut self,
        processed_ep: &ProcessedEffectParam,
        launcher_stats: &Stats,
        is_crit: bool,
        current_turn: usize, // to process aggro
    ) -> EffectOutcome {
        // eval if the effect can be applied on the target
        if processed_ep.input_effect_param.buffer.stats_name.is_empty()
            || !self
                .stats
                .all_stats
                .contains_key(&processed_ep.input_effect_param.buffer.stats_name)
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the stat {} does not exist.",
                processed_ep.input_effect_param.buffer.kind,
                self.id_name,
                processed_ep.input_effect_param.buffer.stats_name
            );
            return EffectOutcome {
                target_id_name: self.id_name.clone(),
                ..Default::default()
            };
        }

        // eval `full_amount`
        let mut full_amount;
        let mut processed_effect_param = processed_ep.clone();
        let pow_current =
            launcher_stats.get_power_stat(processed_ep.input_effect_param.is_magic_atk);
        if processed_ep.input_effect_param.buffer.stats_name == HP
            && processed_ep.input_effect_param.buffer.kind == BufKinds::DecreasingRateOnTurn
        {
            // prepare for HOT
            full_amount = processed_ep.number_of_applies
                * (processed_ep.input_effect_param.buffer.value
                    + pow_current / processed_ep.input_effect_param.nb_turns);
            // update effect value
            processed_effect_param.input_effect_param.buffer.value = full_amount;
        } else if processed_ep.input_effect_param.buffer.stats_name == HP
            && processed_ep.input_effect_param.buffer.kind == BufKinds::ChangeCurrentStatByValue
        {
            if processed_ep.input_effect_param.buffer.value > 0 {
                // HOT
                full_amount = processed_ep.number_of_applies
                    * (processed_ep.input_effect_param.buffer.value + pow_current)
                    / processed_ep.input_effect_param.nb_turns;
            } else {
                // DOT
                full_amount = processed_ep.number_of_applies
                    * AttackType::damage_by_atk(
                        &self.stats,
                        launcher_stats,
                        processed_ep.input_effect_param.is_magic_atk,
                        processed_ep.input_effect_param.buffer.value,
                        processed_ep.input_effect_param.nb_turns,
                    );
            }
        } else if processed_ep.input_effect_param.buffer.kind
            == BufKinds::ChangeCurrentStatByPercentage
            && Stats::is_energy_stat(&processed_ep.input_effect_param.buffer.stats_name)
        {
            full_amount = processed_ep.number_of_applies
                * self
                    .stats
                    .all_stats
                    .get(&processed_ep.input_effect_param.buffer.stats_name)
                    .unwrap()
                    .max as i64
                * processed_ep.input_effect_param.buffer.value
                / 100;
        } else {
            full_amount =
                processed_ep.number_of_applies * processed_ep.input_effect_param.buffer.value;
        }
        // Apply buf/debuf, crit, blocking on damages/heal
        if processed_ep.input_effect_param.buffer.stats_name == HP {
            full_amount = self.character_rounds_info.apply_buf_debuf(
                full_amount,
                &processed_ep.input_effect_param.target_kind,
                is_crit,
            );
            processed_effect_param.input_effect_param.buffer.value = full_amount;
        }
        // blocking the atk
        if self
            .character_rounds_info
            .is_blocking(&processed_ep.input_effect_param)
        {
            full_amount = 10 * full_amount / 100;
        }

        // Process stats `HP`
        // Calculation of the real amount of the value of the effect and update the energy stats
        let real_hp_amount = self
            .stats
            .update_hp_process_real_amount(&processed_ep.input_effect_param, full_amount);

        // Apply the effect on the target
        let real_dmg_amount = self.apply_effect_full_amount(processed_ep, full_amount);

        // output real dmg amount for dmg and heal
        let real_dmg_amount = if real_dmg_amount < 0 {
            real_dmg_amount
        } else {
            real_hp_amount
        };

        // process aggro for `HP` and `non-HP` stats
        let mut aggro_generated: u64 = 0;
        if processed_ep.input_effect_param.buffer.kind != BufKinds::ChangeMaxStatByValue
            && processed_ep.input_effect_param.buffer.kind != BufKinds::ChangeMaxStatByPercentage
        {
            if processed_ep.input_effect_param.buffer.stats_name == HP {
                // process aggro for the launcher
                aggro_generated = self.process_aggro(real_hp_amount, 0, current_turn);
            } else {
                // Add aggro to a target
                aggro_generated = self.process_aggro(
                    0,
                    processed_ep.input_effect_param.buffer.value,
                    current_turn,
                );
            }
        }

        // update stats in game
        EffectOutcome {
            full_amount_tx: full_amount,
            real_amount_tx: real_dmg_amount,
            target_id_name: self.id_name.clone(),
            is_critical: is_crit,
            aggro_generated,
        }
    }

    /// Apply the effect on the target and return the real amount of dmg change if the effect is on hp, otherwise return None
    fn apply_effect_full_amount(
        &mut self,
        processed_ep: &ProcessedEffectParam,
        full_amount: i64,
    ) -> i64 {
        // Process non-stats `HP`
        // Otherwise update the max value of the stats
        if processed_ep.input_effect_param.buffer.stats_name != HP
            && (processed_ep.input_effect_param.buffer.kind == BufKinds::ChangeMaxStatByPercentage
                || processed_ep.input_effect_param.buffer.kind == BufKinds::ChangeMaxStatByValue)
        {
            self.stats.set_stats_on_effect(
                &processed_ep.input_effect_param.buffer.stats_name,
                full_amount,
                processed_ep.input_effect_param.buffer.kind == BufKinds::ChangeMaxStatByPercentage,
                true,
            );
        }
        // apply change current stats for non HP stats
        let mut overhead_dmg = 0;
        if processed_ep.input_effect_param.buffer.stats_name != HP
            && processed_ep.input_effect_param.buffer.kind == BufKinds::ChangeCurrentStatByValue
        {
            overhead_dmg = self.stats.modify_stat_current(
                &processed_ep.input_effect_param.buffer.stats_name,
                full_amount,
            );
        }
        full_amount - overhead_dmg
    }

    pub fn process_atk(
        &mut self,
        game_state: &GameState,
        is_crit: bool,
        atk: &AttackType,
    ) -> Result<Vec<ProcessedEffectParam>> {
        self.process_all_effects(game_state, is_crit, &atk.name, &atk.all_effects)
    }

    fn process_all_effects(
        &mut self,
        game_state: &GameState,
        is_crit: bool,
        action_name: &str,
        all_effects: &[EffectParam],
    ) -> Result<Vec<ProcessedEffectParam>> {
        let mut processed_effect_param_list: Vec<ProcessedEffectParam> = vec![];
        for effect in all_effects {
            processed_effect_param_list.push(self.character_rounds_info.process_one_effect(
                effect,
                action_name,
                game_state,
                is_crit,
            )?);
        }
        Ok(processed_effect_param_list)
    }

    /// Update the aggro of the character by the atkValue or the input aggro value and return the generated aggro
    pub fn process_aggro(&mut self, atk_value: i64, aggro_value: i64, turn_nb: usize) -> u64 {
        let aggro_norm = 20.0;
        let mut local_aggro = aggro_value as f64;
        // Aggro filled by atkValue or input aggro value ?
        if atk_value != 0 {
            local_aggro = (atk_value.abs() as f64 / aggro_norm).round();
        }
        // case null aggro
        if local_aggro == 0.0 {
            return 0;
        }

        // Update aggro
        if let Some(aggro_stat) = self.stats.all_stats.get_mut(AGGRO)
            && let Some(tx_map) = self
                .character_rounds_info
                .tx_rx
                .get_mut(AmountType::Aggro as usize)
            && let Some(aggro) = tx_map.get_mut(&(turn_nb as u64))
        {
            // update txrx current turn nb
            *aggro += local_aggro as i64;
            // update stats aggro of character
            aggro_stat.current = aggro_stat.current.saturating_add(*aggro as u64);
        }
        local_aggro as u64
    }

    pub fn is_receiving_atk(
        &mut self,
        processed_ep: &ProcessedEffectParam,
        game_state: &GameState,
        is_crit: bool,
        launcher_info: &LauncherAtkInfo,
    ) -> (Option<GameAtkEffect>, Option<Vec<DodgeInfo>>) {
        let mut option_gae: Option<GameAtkEffect> = None;
        let mut di: Vec<DodgeInfo> = Vec::new();
        if self.stats.is_dead() == Some(true) {
            tracing::info!("is_receiving_atk: {} is already dead.", self.id_name);
            return (None, None);
        }

        let target_data = TargetData {
            launcher_id_name: launcher_info.id_name.to_string(),
            target_id_name: self.id_name.clone(),
            target_chara_kind: self.kind.clone(),
            launcher_chara_kind: launcher_info.kind.clone(),
            effect_param: processed_ep.input_effect_param.clone(),
        };

        // check if the effect is applied on the target
        if self.character_rounds_info.is_effect_applied(&target_data) {
            let effect_outcome = self.apply_processed_effect_param(
                processed_ep,
                &launcher_info.stats,
                is_crit,
                game_state.current_turn_nb,
            );
            // assess the blocking
            if self
                .character_rounds_info
                .is_blocking(&processed_ep.input_effect_param)
            {
                di.push(self.character_rounds_info.dodge_info.clone());
            }
            // update all effects
            let gae = GameAtkEffect {
                processed_effect_param: processed_ep.clone(),
                atk_type: launcher_info.atk_type.clone(),
                launching_turn: game_state.current_turn_nb,
                launching_round: game_state.current_round,
                effect_outcome: effect_outcome.clone(),
            };
            // update character table of effects when the effect takes place
            self.character_rounds_info.all_effects.push(gae.clone());
            // update stats table
            option_gae = Some(gae.clone());
        } else {
            tracing::info!(
                "is_receiving_atk: effect is not applied on:{} current_turn:{}, current_round:{}, kind:{:?}, launcher_info.id_name:{}, effect.target: {:?}, launcher_kind: {:?}, effect.type: {:?}, effect.stats_name: {}.",
                self.id_name,
                game_state.current_turn_nb,
                game_state.current_round,
                self.kind,
                launcher_info.id_name,
                processed_ep.input_effect_param.target_kind,
                launcher_info.kind,
                processed_ep.input_effect_param.buffer.kind,
                processed_ep.input_effect_param.buffer.stats_name
            );
        }
        // assess the dodging
        if self
            .character_rounds_info
            .is_dodging(&processed_ep.input_effect_param.target_kind)
            && self.kind != launcher_info.kind
            && self.character_rounds_info.is_current_target
        {
            tracing::info!("{:?}", self.character_rounds_info.dodge_info);
            di.push(self.character_rounds_info.dodge_info.clone());
        }

        let all_dodging = (!di.is_empty()).then_some(di);
        (option_gae, all_dodging)
    }

    /// The attak can be launched if the character has enough mana, vigor and
    /// berserk and if the atk is not under a cooldown.
    /// If the atk can be launched, true is returned, otherwise false is returned.
    pub fn can_be_launched(&self, atk_type: &AttackType) -> bool {
        // needed level too high
        if self.level < atk_type.level {
            return false;
        }

        // that attack has a cooldown
        for atk_effect in &atk_type.all_effects {
            if atk_effect.buffer.kind == BufKinds::CooldownTurnsNumber {
                for e in &self.character_rounds_info.all_effects {
                    if e.atk_type.name == atk_type.name
                        && e.processed_effect_param.input_effect_param.buffer.value
                            - e.processed_effect_param.counter_turn
                            > 0
                    {
                        return false;
                    }
                }
            }

            if atk_effect.buffer.stats_name == HP
                && (atk_effect.target_kind == TARGET_ALLY
                    || atk_effect.target_kind == TARGET_ONLY_ALLY
                    || atk_effect.target_kind == TARGET_ALL_ALLIES)
                && self.character_rounds_info.is_heal_atk_blocked
            {
                return false;
            }
        }

        // atk cost enough ?
        let mana = &self.stats.all_stats[MANA];
        let vigor = &self.stats.all_stats[VIGOR];
        let berserk = &self.stats.all_stats[BERSERK];

        if (atk_type.mana_cost > 0 && !self.has_energy_kind(&EnergyKind::Mana))
            || (atk_type.vigor_cost > 0 && !self.has_energy_kind(&EnergyKind::Vigor))
            || (atk_type.berseck_cost > 0 && !self.has_energy_kind(&EnergyKind::Berserk))
        {
            return false;
        }

        atk_type.mana_cost * mana.max / 100 <= mana.current
            && atk_type.vigor_cost * vigor.max / 100 <= vigor.current
            && atk_type.berseck_cost <= berserk.current
    }

    pub fn apply_hot_or_dot(&mut self, current_turn_nb: usize, hot_or_dot: i64) -> String {
        let mut log = String::new();
        if hot_or_dot != 0 {
            let overhead = self.stats.modify_stat_current(HP, hot_or_dot);

            // TODO output log
            // localLog.append(QString("HOT et DOT totaux: %1").arg(hotAndDot));
            // update buf overheal
            if overhead > 0 {
                // update txrx
                self.character_rounds_info.tx_rx[AmountType::OverHealRx as usize]
                    .insert(current_turn_nb as u64, overhead);
                log = format!("overheal of {}", overhead);
            }
        }
        log
    }

    pub fn new_round(
        &mut self,
        current_turn_nb: usize,
        launchable_atks: Vec<AttackType>,
    ) -> Vec<LogData> {
        let mut output_logs_data: Vec<LogData> = Vec::new();
        self.character_rounds_info.actions_done_in_round = 0;

        if self.character_rounds_info.is_first_round {
            self.character_rounds_info.is_first_round = false;
            // aggro is initialized before any action
            self.init_aggro_on_turn(current_turn_nb);

            match self.remove_terminated_effect_on_player() {
                Ok(effects_param_removed) => effects_param_removed.iter().for_each(|e| {
                    output_logs_data.push(LogData {
                        message: format!("{} on {}", e.buffer.kind, e.buffer.stats_name),
                        ..Default::default()
                    })
                }),
                Err(e) => output_logs_data.push(LogData {
                    message: format!("effects not removed on {}: {}", self.id_name, e),
                    color: DARK_RED.to_string(),
                }),
            }

            // TODO apply passive power

            // atk assessment to be launched
            self.character_rounds_info
                .apply_launchable_atks(launchable_atks);

            // apply hot and dot
            let (mut process_logs, hot_or_dot) = self
                .character_rounds_info
                .process_hot_and_dot(current_turn_nb);
            output_logs_data.append(&mut process_logs);
            let hot_dot_logs = self.apply_hot_or_dot(current_turn_nb, hot_or_dot);
            output_logs_data.push(LogData {
                message: hot_dot_logs,
                color: LIGHT_GREEN.to_string(),
            });
        }
        output_logs_data
    }

    pub fn is_boss_atk(&self) -> bool {
        self.kind == CharacterKind::Boss
    }

    pub fn use_consumable(
        &mut self,
        consumable: Consumable,
        game_state: &GameState,
        launcher_stats: &Stats,
    ) -> Result<Vec<EffectOutcome>> {
        if !self.inventory.contains_potion(&consumable.name) {
            bail!("no {} is in the inventory", consumable.name)
        }
        match self.process_all_effects(game_state, false, &consumable.name, &consumable.effects) {
            Ok(all_processed_ep) => {
                let mut all_eo: Vec<EffectOutcome> = vec![];
                for processed_ep in all_processed_ep {
                    all_eo.push(self.apply_processed_effect_param(
                        &processed_ep,
                        launcher_stats,
                        false,
                        game_state.current_turn_nb,
                    ));
                }
                self.inventory.remove_potion(&consumable.name);
                Ok(all_eo)
            }
            Err(e) => Err(e),
        }
    }

    pub fn toggle_equipment(
        &mut self,
        new_equipment_unique_name: &str,
        all_equipments: &HashMap<EquipmentJsonKey, Vec<Equipment>>,
    ) {
        // downdate stats of previous equipment if exist
        let equipment_off: HashMap<EquipmentJsonKey, Vec<Equipment>> =
            self.inventory.get_all_equipments(
                all_equipments
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<Equipment>>()
                    .as_slice(),
                true,
            );
        self.stats.remove_equipment_on_stats(
            &equipment_off
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Equipment>>(),
        );

        // toggle equipment
        self.inventory.toggle_equipment(new_equipment_unique_name);

        // update stats of new equipment
        let equipment_on: HashMap<EquipmentJsonKey, Vec<Equipment>> =
            self.inventory.get_all_equipments(
                all_equipments
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<Equipment>>()
                    .as_slice(),
                true,
            );
        self.stats.apply_equipment_on_stats(
            &equipment_on
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Equipment>>(),
        );
        // apply the effects
        self.apply_effects_on_stats(false);
    }

    fn apply_effects_on_stats(&mut self, update_effect_stats: bool) {
        self.character_rounds_info
            .all_effects
            .iter_mut()
            .for_each(|gae| {
                if gae.processed_effect_param.input_effect_param.buffer.kind
                    == BufKinds::ChangeMaxStatByPercentage
                    || gae.processed_effect_param.input_effect_param.buffer.kind
                        == BufKinds::ChangeMaxStatByValue
                {
                    self.stats.set_stats_on_effect(
                        &gae.processed_effect_param
                            .input_effect_param
                            .buffer
                            .stats_name,
                        gae.effect_outcome.full_amount_tx,
                        gae.processed_effect_param.input_effect_param.buffer.kind
                            == BufKinds::ChangeMaxStatByPercentage,
                        update_effect_stats,
                    );
                }
            });
    }

    pub fn has_energy_kind(&self, energy_kind: &EnergyKind) -> bool {
        self.energies.iter().any(|e| &e.kind == energy_kind)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use strum::IntoEnumIterator;

    use super::Character;
    use crate::character_mod::attack_type::AttackType;
    use crate::character_mod::buffers::Buffer;
    use crate::character_mod::character::AmountType;
    use crate::character_mod::effect::EffectOutcome;
    use crate::character_mod::effect::{Condition, ConditionKind};
    use crate::character_mod::energy::EnergyKind;
    use crate::character_mod::equipment::{Equipment, EquipmentJsonKey};
    use crate::common::constants::paths_const::TEST_OFFLINE_ROOT;
    use crate::server::players_manager::GameAtkEffect;
    use crate::testing::testing_all_characters::{self, testing_all_equipment, testing_character};
    use crate::{
        character_mod::buffers::BufKinds,
        character_mod::character::{CharacterKind, Class},
        character_mod::effect::EffectParam,
        common::constants::stats_const::*,
        testing::testing_effect::*,
    };

    #[test]
    fn unit_try_new_from_json() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let equipment = testing_all_equipment();
        assert_eq!(EquipmentJsonKey::iter().count(), equipment.len());
        let c = Character::try_new_from_json(file_path, *TEST_OFFLINE_ROOT, false, &equipment);
        assert!(c.is_ok());
        let c = c.unwrap();
        // name
        assert_eq!("test", c.db_full_name);
        assert_eq!("test", c.short_name);
        assert_eq!("test_#1", c.id_name);
        // buf-debuf
        assert_eq!(3, c.character_rounds_info.all_buffers.len());
        assert_eq!(
            BufKinds::DamageRxPercent,
            c.character_rounds_info.all_buffers[0].kind
        );
        assert!(!c.character_rounds_info.all_buffers[0].is_passive_enabled);
        assert!(c.character_rounds_info.all_buffers[0].is_percent);
        assert_eq!(100, c.character_rounds_info.all_buffers[0].value);
        assert_eq!(
            BufKinds::NextHealAtkIsCrit,
            c.character_rounds_info.all_buffers[1].kind
        );
        assert!(c.character_rounds_info.all_buffers[1].is_passive_enabled);
        assert!(!c.character_rounds_info.all_buffers[1].is_percent);
        assert_eq!(0, c.character_rounds_info.all_buffers[1].value);
        assert_eq!(
            BufKinds::ChangeCurrentStatByValue,
            c.character_rounds_info.all_buffers[2].kind
        );
        assert!(c.character_rounds_info.all_buffers[2].is_passive_enabled);
        assert!(!c.character_rounds_info.all_buffers[2].is_percent);
        assert_eq!(10, c.character_rounds_info.all_buffers[2].value);
        // Class
        assert_eq!(Class::Standard, c.class);
        // Color
        assert_eq!("green", c.color_theme);
        // Experience
        assert_eq!(50, c.character_rounds_info.exp);
        // extended character
        assert!(c.character_rounds_info.is_first_round);
        assert!(c.character_rounds_info.is_heal_atk_blocked);
        assert!(!c.character_rounds_info.is_random_target);
        // level
        assert_eq!(1, c.level);
        // photo
        assert_eq!("phototest", c.photo_name);
        // stats
        // stats - aggro
        assert_eq!(0, c.stats.all_stats[AGGRO].current);
        assert_eq!(10009, c.stats.all_stats[AGGRO].max); // buffer on aggro, 9999 -> 10009
        // stats - aggro test init stats only on aggro
        assert_eq!(
            c.stats.all_stats[AGGRO].current_raw,
            c.stats.all_stats[AGGRO].current
        );
        assert_eq!(c.stats.all_stats[AGGRO].max_raw, 9999);
        // stats - aggro rate
        assert_eq!(1, c.stats.all_stats[AGGRO_RATE].current);
        assert_eq!(1, c.stats.all_stats[AGGRO_RATE].max);
        // stats - berseck
        assert_eq!(105, c.stats.all_stats[BERSERK].current);
        assert_eq!(210, c.stats.all_stats[BERSERK].max); // right ring + 10 to max berseck (ratio -> update current 100 -> 105)
        // stats - berseck_rate
        assert_eq!(1, c.stats.all_stats[BERSECK_RATE].current); // +4 right ring
        assert_eq!(1, c.stats.all_stats[BERSECK_RATE].max);
        // stats - critical_strike
        assert_eq!(10, c.stats.all_stats[CRITICAL_STRIKE].current);
        assert_eq!(10, c.stats.all_stats[CRITICAL_STRIKE].max);
        // stats - dodge
        assert_eq!(29, c.stats.all_stats[DODGE].current);
        assert_eq!(29, c.stats.all_stats[DODGE].max);
        // stats - hp
        assert_eq!(1, c.stats.all_stats[HP].current);
        assert_eq!(135, c.stats.all_stats[HP].max);
        assert_eq!(135, c.stats.all_stats[HP].max_raw);
        assert_eq!(1, c.stats.all_stats[HP].current_raw);
        // stats - hp_regeneration
        assert_eq!(7, c.stats.all_stats[HP_REGEN].current);
        assert_eq!(7, c.stats.all_stats[HP_REGEN].max);
        // stats - magic_armor
        assert_eq!(15, c.stats.all_stats[MAGICAL_ARMOR].current);
        assert_eq!(15, c.stats.all_stats[MAGICAL_ARMOR].max);
        // stats - magic_power
        assert_eq!(30, c.stats.all_stats[MAGICAL_POWER].current);
        assert_eq!(30, c.stats.all_stats[MAGICAL_POWER].max);
        // stats - mana
        assert_eq!(210, c.stats.all_stats[MANA].current);
        assert_eq!(210, c.stats.all_stats[MANA].max);
        // stats - mana_regeneration
        assert_eq!(7, c.stats.all_stats[MANA_REGEN].current);
        assert_eq!(7, c.stats.all_stats[MANA_REGEN].max);
        // stats - physical_armor
        assert_eq!(30, c.stats.all_stats[PHYSICAL_ARMOR].current);
        assert_eq!(30, c.stats.all_stats[PHYSICAL_ARMOR].max);
        // stats - physical_power
        assert_eq!(40, c.stats.all_stats[PHYSICAL_POWER].current);
        assert_eq!(40, c.stats.all_stats[PHYSICAL_POWER].max);
        // stats - speed
        assert_eq!(212, c.stats.all_stats[SPEED].current);
        assert_eq!(212, c.stats.all_stats[SPEED].max);
        // stats - speed_regeneration
        assert_eq!(12, c.stats.all_stats[SPEED_REGEN].current);
        assert_eq!(12, c.stats.all_stats[SPEED_REGEN].max); // + 10% by amulet
        // stats - vigor
        assert_eq!(210, c.stats.all_stats[VIGOR].current);
        assert_eq!(210, c.stats.all_stats[VIGOR].max);
        // stats - vigor_regeneration
        assert_eq!(5, c.stats.all_stats[VIGOR_REGEN].current);
        assert_eq!(5, c.stats.all_stats[VIGOR_REGEN].max);
        // tx-rx
        assert_eq!(7, c.character_rounds_info.tx_rx.len());
        // Type - kind
        assert_eq!(CharacterKind::Hero, c.kind);
        // nb-actions-in-round
        assert_eq!(0, c.character_rounds_info.actions_done_in_round);
        // atk
        assert_eq!(16, c.attacks_list.len());
        // equipment
        assert_eq!(
            13,
            c.inventory
                .get_all_equipments(
                    equipment
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<Equipment>>()
                        .as_slice(),
                    true
                )
                .len()
        );
        // energy
        assert_eq!(3, c.energies.len());
        assert_eq!(c.energies[0].kind, EnergyKind::Mana.to_owned());
        assert_eq!(c.energies[1].kind, EnergyKind::Vigor.to_owned());
        assert_eq!(c.energies[2].kind, EnergyKind::Berserk.to_owned());

        let file_path = "./tests/offlines/characters/wrong.json";
        assert!(
            Character::try_new_from_json(
                file_path,
                *TEST_OFFLINE_ROOT,
                false,
                &testing_all_equipment()
            )
            .is_err()
        );
    }

    #[test]
    fn unit_init_aggro_on_turn() {
        let mut c = Character::default();
        c.stats.init();
        c.init_aggro_on_turn(1);
        assert_eq!(0, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx.push(HashMap::new());
        c.character_rounds_info.tx_rx.push(HashMap::new());
        c.character_rounds_info.tx_rx.push(HashMap::new());
        c.character_rounds_info.tx_rx.push(HashMap::new());
        c.character_rounds_info.tx_rx.push(HashMap::new());
        c.character_rounds_info.tx_rx.push(HashMap::new());
        c.character_rounds_info.tx_rx[5].insert(1, 10);
        c.init_aggro_on_turn(2);
        assert_eq!(10, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx[5].insert(2, 20);
        c.init_aggro_on_turn(3);
        assert_eq!(30, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx[5].insert(3, 30);
        c.init_aggro_on_turn(4);
        assert_eq!(60, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx[5].insert(4, 40);
        c.init_aggro_on_turn(5);
        assert_eq!(100, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx[5].insert(5, 50);
        c.init_aggro_on_turn(6);
        assert_eq!(150, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx[5].insert(6, 60);
        c.init_aggro_on_turn(7);
        assert_eq!(200, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx[5].insert(7, 70);
        c.init_aggro_on_turn(8);
        assert_eq!(250, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx[5].insert(8, 80);
        c.init_aggro_on_turn(9);
        assert_eq!(300, c.stats.all_stats[AGGRO].current);
        c.character_rounds_info.tx_rx[5].insert(9, 90);
        c.init_aggro_on_turn(10);
        assert_eq!(350, c.stats.all_stats[AGGRO].current);
    }

    #[test]
    fn unit_remove_malus_effect() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let c = Character::try_new_from_json(
            file_path,
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        );
        assert!(c.is_ok());
        let mut c = c.unwrap();
        let ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::ChangeMaxStatByValue,
                stats_name: HP.to_string(),
                value: -10,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = c.remove_malus_effect(&ep);
        assert_eq!(145, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        let ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::ChangeMaxStatByPercentage,
                stats_name: HP.to_string(),
                value: -10,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(result.is_ok());

        let result = c.remove_malus_effect(&ep);
        assert!(result.is_ok());
        assert_eq!(158, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        let ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::BlockHealAtk,
                stats_name: HP.to_string(),
                value: 10,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = c.remove_malus_effect(&ep);
        assert!(result.is_ok());
        assert!(!c.character_rounds_info.is_heal_atk_blocked);

        // remove EFFECT_CHANGE_RX_DAMAGES_BY_PERCENT
        let ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::DamageRxPercent,
                stats_name: HP.to_string(),
                value: 10,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(result.is_ok());

        let result = c.remove_malus_effect(&ep);
        assert!(result.is_ok());
        assert_eq!(
            90, // from 100 in test.json
            c.character_rounds_info
                .get_buffer_by_type(&BufKinds::DamageRxPercent)
                .as_ref()
                .unwrap()
                .value
        );
    }

    #[test]
    fn unit_process_one_effect() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let c = Character::try_new_from_json(
            file_path,
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        );
        assert!(c.is_ok());
        let mut c = c.unwrap();
        let mut ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::CooldownTurnsNumber,
                value: 10,
                ..Default::default()
            },
            nb_turns: 1,
            target_kind: c.id_name.clone(),
            ..Default::default()
        };
        let mut game_state = Default::default();
        // target is himself
        let processed_effect_param = c
            .character_rounds_info
            .process_one_effect(&ep, "", &game_state, false)
            .unwrap();
        assert_eq!(
            BufKinds::CooldownTurnsNumber,
            processed_effect_param.input_effect_param.buffer.kind
        );
        assert_eq!(10, processed_effect_param.input_effect_param.buffer.value);
        assert_eq!(
            c.id_name,
            processed_effect_param.input_effect_param.target_kind
        );
        assert_eq!(10, processed_effect_param.input_effect_param.buffer.value);
        assert_eq!(
            0,
            processed_effect_param.input_effect_param.sub_value_effect
        );
        assert_eq!(
            "Cooldown actif sur  de 10 tours.",
            processed_effect_param.log.message
        );

        // test - critical
        ep.buffer.stats_name = HP.to_owned();
        ep.buffer.kind = BufKinds::ChangeMaxStatByValue;
        ep.buffer.value = 10;
        let processed_effect_param = c
            .character_rounds_info
            .process_one_effect(&ep, "", &game_state, true)
            .unwrap();
        assert_eq!(
            BufKinds::ChangeMaxStatByValue,
            processed_effect_param.input_effect_param.buffer.kind
        );
        assert_eq!(1, processed_effect_param.input_effect_param.nb_turns);
        // crit : 10 -> 15
        assert_eq!(15, processed_effect_param.input_effect_param.buffer.value);
        assert_eq!(
            c.id_name,
            processed_effect_param.input_effect_param.target_kind
        );
        assert_eq!(
            0,
            processed_effect_param.input_effect_param.sub_value_effect
        );
        assert_eq!(
            "Max stat of HP is up by value:15",
            processed_effect_param.log.message
        );

        // conditions - number of died ennemies
        game_state.current_turn_nb = 1;
        game_state.died_ennemies.insert(0, vec!["".to_owned()]);
        ep.conditions.push(Condition {
            kind: ConditionKind::NbEnnemiesDied,
            ..Default::default()
        });
        ep.sub_value_effect = 10;
        ep.buffer.value = 0;
        let processed_effect_param = c
            .character_rounds_info
            .process_one_effect(&ep, "", &game_state, false)
            .unwrap();
        // focus on effect_type
        assert_eq!(
            BufKinds::ChangeMaxStatByValue,
            processed_effect_param.input_effect_param.buffer.kind
        );
        assert_eq!(1, processed_effect_param.input_effect_param.nb_turns);
        assert_eq!(
            c.id_name,
            processed_effect_param.input_effect_param.target_kind
        );
        // focus on value
        assert_eq!(0, processed_effect_param.input_effect_param.buffer.value);
        assert_eq!(
            10,
            processed_effect_param.input_effect_param.sub_value_effect
        );
        assert_eq!(
            "Max stat of HP is up by value:0",
            processed_effect_param.log.message
        );
    }

    #[test]
    fn unit_remove_terminated_effect_on_player() {
        let mut c = testing_character();
        c.character_rounds_info
            .all_effects
            .push(GameAtkEffect::default());
        c.remove_terminated_effect_on_player().unwrap();
        assert_eq!(0, c.character_rounds_info.all_effects.len());
    }

    #[test]
    fn unit_process_atk_cost() {
        let mut c = testing_character();

        let old_vigor_current = c.stats.all_stats[VIGOR].current;
        let old_mana_current = c.stats.all_stats[MANA].current;
        let old_berseck_current = c.stats.all_stats[BERSERK].current;
        let old_vigor_max = c.stats.all_stats[VIGOR].max;
        let old_mana_max = c.stats.all_stats[MANA].max;
        let old_berseck_max = c.stats.all_stats[BERSERK].max;
        c.process_atk_cost("atk1"); // 10% vigor cost

        assert_eq!(
            old_vigor_current - 10 * old_vigor_max / 100,
            c.stats.all_stats[VIGOR].current
        );
        assert_eq!(
            old_mana_current - 10 * old_mana_max / 100,
            c.stats.all_stats[MANA].current
        );
        assert_eq!(
            old_berseck_current - 10 * old_berseck_max / 100,
            c.stats.all_stats[BERSERK].current
        );
        c.process_atk_cost("atk1"); // 10% vigor cost again!
        assert_eq!(
            old_vigor_current - 20 * old_vigor_max / 100,
            c.stats.all_stats[VIGOR].current
        );
        assert_eq!(
            old_mana_current - 20 * old_mana_max / 100,
            c.stats.all_stats[MANA].current
        );
        assert_eq!(
            old_berseck_current - 20 * old_berseck_max / 100,
            c.stats.all_stats[BERSERK].current
        );
    }

    #[test]
    fn unit_process_dodging() {
        let mut c = testing_character();

        // ultimate atk cannot be dodged
        let atk_level = 13;
        c.process_dodging(atk_level);
        assert!(!c.character_rounds_info.dodge_info.is_dodging);
        assert!(!c.character_rounds_info.dodge_info.is_blocking);

        // impossible to dodge
        let atk_level = 1;
        c.stats.all_stats[DODGE].current = 0;
        c.process_dodging(atk_level);
        assert!(!c.character_rounds_info.dodge_info.is_dodging);
        assert!(!c.character_rounds_info.dodge_info.is_blocking);

        // total dodge
        let atk_level = 1;
        c.stats.all_stats[DODGE].current = 100;
        c.process_dodging(atk_level);
        assert!(c.character_rounds_info.dodge_info.is_dodging);
        assert!(!c.character_rounds_info.dodge_info.is_blocking);

        // A tank is not dodging, he is blocking
        let atk_level = 1;
        c.stats.all_stats[DODGE].current = 100;
        c.class = Class::Berserker;
        c.process_dodging(atk_level);
        assert!(!c.character_rounds_info.dodge_info.is_dodging);
        assert!(c.character_rounds_info.dodge_info.is_blocking);
    }

    #[test]
    fn unit_process_critical_strike() {
        // no critical strike buff
        let mut c = testing_character();
        c.stats.all_stats[CRITICAL_STRIKE].current = 0;
        assert!(!c.process_critical_strike("atk1").unwrap());
        // ensure critical strike
        c.stats.all_stats[CRITICAL_STRIKE].current = 100;
        assert!(c.process_critical_strike("atk1").unwrap());
        assert!(
            c.character_rounds_info
                .get_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
                .as_ref()
                .unwrap()
                .is_passive_enabled
        );

        // critical strike is processed only on atk with heal effect
        assert!(c.process_critical_strike("atk_heal1_indiv").unwrap());
        assert!(
            !c.character_rounds_info
                .get_buffer_by_type(&BufKinds::NextHealAtkIsCrit)
                .as_ref()
                .unwrap()
                .is_passive_enabled
        );
    }

    #[test]
    fn unit_apply_effect_outcome() {
        let mut c = Character::try_new_from_json(
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
        let mut processed_ep = build_cooldown_effect();
        let launcher_stats = c.stats.clone();
        // target is himself
        let eo = c.apply_processed_effect_param(&processed_ep, &launcher_stats, false, 0);
        assert_eq!(
            eo,
            EffectOutcome {
                full_amount_tx: 0,
                real_amount_tx: 0,
                target_id_name: c.id_name.clone(),
                is_critical: false,
                aggro_generated: 0,
            }
        );

        // target is other ally
        processed_ep = build_hot_effect_individual();
        let old_hp = c2.stats.all_stats[HP].current;
        let eo = c2.apply_processed_effect_param(&processed_ep, &launcher_stats, false, 0);
        assert_eq!(eo.full_amount_tx, 35);
        assert_eq!(eo.real_amount_tx, 35);
        assert_eq!(old_hp + 35, c2.stats.all_stats[HP].current);

        // target is ennemy
        let mut boss1 = Character::try_new_from_json(
            "./tests/offlines/characters/test_boss1.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        // dmg: -30(dmg) - 10(phy pow stats character) -10*7(phy pow equipment) 0 -70
        // * 1000/1000+ [5(def phy armor) + 25] = 0.97
        // => -70 * 0.97 = -67.9 ~ -68
        processed_ep = build_dmg_effect_individual();
        let old_hp = boss1.stats.all_stats[HP].current;
        let eo = boss1.apply_processed_effect_param(&processed_ep, &launcher_stats, false, 0);
        assert_eq!(eo.full_amount_tx, -68);
        assert_eq!(eo.real_amount_tx, -68);
        assert_eq!(old_hp - 68, boss1.stats.all_stats[HP].current);

        processed_ep = build_buf_effect_individual_speed_regen();
        let launcher_stats = c.stats.clone();
        let eo = c.apply_processed_effect_param(&processed_ep, &launcher_stats, false, 0);
        assert_eq!(eo.full_amount_tx, 60);
        assert_eq!(eo.real_amount_tx, 0);
        assert_eq!(eo.aggro_generated, 0);
    }

    #[test]
    fn unit_process_aggro() {
        let mut c = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        c.init_aggro_on_turn(0);
        let aggro_generated = c.process_aggro(0, 0, 0);
        assert_eq!(
            0,
            c.character_rounds_info.tx_rx[AmountType::Aggro as usize][&0]
        );
        assert_eq!(0, aggro_generated);

        let aggro_generated = c.process_aggro(20, 0, 0);
        assert_eq!(
            1,
            c.character_rounds_info.tx_rx[AmountType::Aggro as usize][&0]
        );
        assert_eq!(1, aggro_generated);
    }

    #[test]
    fn unit_reset_all_effects_on_player() {
        let mut c = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        let hp_without_malus = c.stats.all_stats[HP].max as i64;
        c.character_rounds_info.all_effects.push(GameAtkEffect {
            processed_effect_param: build_effect_max_stats(),
            atk_type: AttackType::default(),
            launching_turn: 0,
            launching_round: 0,
            effect_outcome: EffectOutcome::default(),
        });
        let effect_value = c.character_rounds_info.all_effects[0]
            .processed_effect_param
            .input_effect_param
            .buffer
            .value;
        c.reset_all_effects_on_player().unwrap();
        assert_eq!(
            hp_without_malus - effect_value,
            c.stats.all_stats[HP].max as i64
        );
        assert!(c.character_rounds_info.all_effects.is_empty());
    }

    #[test]
    fn unit_can_be_launched() {
        let mut atk_type = self::AttackType::default();
        let mut c1 = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        // nominal case
        atk_type.level = 1;
        atk_type.mana_cost = 0;
        atk_type.vigor_cost = 0;
        atk_type.berseck_cost = 0;
        atk_type.name = "atk_test".to_owned();
        c1.level = 1;
        let result = c1.can_be_launched(&atk_type);
        assert!(result);
        // character level too low
        c1.level = 0;
        let result = c1.can_be_launched(&atk_type);
        assert!(!result);
        // not enough mana
        c1.level = 1;
        atk_type.mana_cost = c1.stats.all_stats[MANA].current + 100;
        let result = c1.can_be_launched(&atk_type);
        assert!(!result);
        // heal atk blocked
        // c1 (test.json heal_atk_blocked = true)
        atk_type
            .all_effects
            .push(build_heal_atk_blocked().input_effect_param);
        atk_type.mana_cost = c1.stats.all_stats[MANA].current / 100;
        let result = c1.can_be_launched(&atk_type);
        assert!(!result);
        c1.character_rounds_info.is_heal_atk_blocked = false;
        // active cooldown
        atk_type.all_effects.clear();
        atk_type
            .all_effects
            .push(build_cooldown_effect().input_effect_param);
        c1.character_rounds_info.all_effects.push(GameAtkEffect {
            processed_effect_param: build_cooldown_effect(),
            atk_type: atk_type.clone(),
            ..Default::default()
        });
        let result = c1.can_be_launched(&atk_type);
        assert!(!result);
        // inactive cooldown
        atk_type.all_effects.clear();
        atk_type
            .all_effects
            .push(build_cooldown_effect().input_effect_param);
        let mut processed_ep = build_cooldown_effect();
        processed_ep.counter_turn = processed_ep.input_effect_param.buffer.value;
        c1.character_rounds_info.all_effects.clear();
        c1.character_rounds_info.all_effects.push(GameAtkEffect {
            processed_effect_param: processed_ep.clone(),
            atk_type: atk_type.clone(),
            ..Default::default()
        });
        let result = c1.can_be_launched(&atk_type);
        assert!(result);
        // not enough berserk
        atk_type.all_effects.clear();
        atk_type
            .all_effects
            .push(build_hot_effect_individual().input_effect_param);
        c1.character_rounds_info.all_effects.clear();
        atk_type.berseck_cost = c1.stats.all_stats[BERSERK].current + 100;
        let result = c1.can_be_launched(&atk_type);
        assert!(!result);
        // not enough vigor
        atk_type.berseck_cost = c1.stats.all_stats[BERSERK].current;
        atk_type.vigor_cost = c1.stats.all_stats[VIGOR].current + 100;
        let result = c1.can_be_launched(&atk_type);
        assert!(!result);
        // enough energy
        atk_type.berseck_cost = c1.stats.all_stats[BERSERK].current;
        atk_type.vigor_cost = c1.stats.all_stats[VIGOR].current / 100;
        atk_type.mana_cost = c1.stats.all_stats[MANA].current / 100;
        let result = c1.can_be_launched(&atk_type);
        assert!(result);

        // no berserk energy and berserk cost > 0
        atk_type.berseck_cost = 100;
        let c2 = Character::try_new_from_json(
            "./tests/offlines/characters/test2.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        let result = c2.can_be_launched(&atk_type);
        assert!(!result);
    }

    #[test]
    fn unit_apply_hot_or_dot() {
        let mut pl = testing_all_characters::testing_pm();
        pl.current_player.stats.all_stats[HP].current = 100;
        pl.current_player.stats.all_stats[HP].max = 100;
        pl.current_player.stats.all_stats[HP].max_raw = 100;
        pl.current_player.stats.all_stats[HP].current_raw = 100;
        // max value is topped, 100 and not 100 + 30
        pl.current_player.apply_hot_or_dot(0, 30);
        assert_eq!(100, pl.current_player.stats.all_stats[HP].current);

        pl.current_player.apply_hot_or_dot(0, -30);
        assert_eq!(70, pl.current_player.stats.all_stats[HP].current);
    }

    #[test]
    fn unit_toggle_equipment() {
        let mut c = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();

        assert_eq!(12, c.stats.all_stats[SPEED_REGEN].max);
        // add one effect on vigor
        c.character_rounds_info.add_effect_on_player(GameAtkEffect {
            processed_effect_param: build_buf_effect_individual_speed_regen(),
            effect_outcome: EffectOutcome {
                full_amount_tx: 20,
                ..Default::default()
            },
            ..Default::default()
        });
        c.apply_effects_on_stats(true);
        assert_eq!(32, c.stats.all_stats[SPEED_REGEN].max);

        // eval mana max - 200 raw + 10 by starting amulet
        assert_eq!(210, c.stats.all_stats[MANA].max);
        assert_eq!(210, c.stats.all_stats[MANA].current);
        assert_eq!(200, c.stats.all_stats[MANA].max_raw);
        // toggle off the same equipment
        let equip = c.inventory.get_equipped_equipments(
            &testing_all_equipment()
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Equipment>>(),
        );
        // eval that the starting amulet is equipped and gives 10 mana
        assert!(
            equip
                .iter()
                .any(|(_, equips)| equips.iter().any(|e| e.unique_name == "starting amulet"))
        );
        assert_eq!(10, c.stats.all_stats[MANA].buf_equip_value);
        c.toggle_equipment("starting amulet", &testing_all_equipment());
        // eval that the starting amulet is not equipped
        let equip = c.inventory.get_equipped_equipments(
            &testing_all_equipment()
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Equipment>>(),
        );
        assert!(
            equip
                .iter()
                .any(|(_, equips)| equips.iter().any(|e| e.unique_name != "starting amulet"))
        );
        // eval mana update
        assert_eq!(0, c.stats.all_stats[MANA].buf_equip_value);
        assert_eq!(
            210 - 10, // ratio = 1 because mana-current = mana-max
            c.stats.all_stats[MANA].current
        );
        assert_eq!(210 - 10, c.stats.all_stats[MANA].max);

        // toggle on
        c.toggle_equipment("starting amulet", &testing_all_equipment());
        // eval that the starting amulet is equipped
        let equip = c.inventory.get_equipped_equipments(
            &testing_all_equipment()
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Equipment>>(),
        );
        assert!(
            equip
                .iter()
                .any(|(_, equips)| equips.iter().any(|e| e.unique_name == "starting amulet"))
        );
        // eval mana update
        assert_eq!(10, c.stats.all_stats[MANA].buf_equip_value);
        assert_eq!(
            210, // ratio = 1 because mana-current = mana-max
            c.stats.all_stats[MANA].current
        );
        assert_eq!(210, c.stats.all_stats[MANA].max);

        // effect still the same
        assert_eq!(32, c.stats.all_stats[SPEED_REGEN].max);
    }
}
