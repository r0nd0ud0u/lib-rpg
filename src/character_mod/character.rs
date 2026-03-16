use anyhow::{Result, anyhow, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, vec};

use crate::{
    character_mod::{
        attack_type::{AttackType, LauncherAtkInfo},
        class::Class,
        effect::{EffectOutcome, EffectParam, ProcessedEffectParam},
        equipment::{Equipment, EquipmentJsonKey, EquipmentJsonValue},
        powers::Powers,
        rounds_information::{AmountType, CharacterRoundsInfo},
        stats::Stats,
        stats_in_game::StatsInGame,
        target::TargetData,
    },
    common::{
        constants::{all_target_const::*, effect_const::*, paths_const::*, stats_const::*},
        log_data::{
            LogData,
            const_colors::{DARK_RED, LIGHT_GREEN},
        },
    },
    server::{
        game_state::GameState,
        players_manager::{DodgeInfo, GameAtkEffects},
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
    /// key: body, value: equipmentName
    pub equipment_on: HashMap<String, Vec<Equipment>>,
    /// key: attak name, value: AttakType struct
    pub attacks_list: IndexMap<String, AttackType>,
    /// Main color theme of the character
    #[serde(rename = "Color")]
    pub color_theme: String,
    /// Powers
    #[serde(rename = "Powers")]
    pub power: Powers,
    /// CharacterRoundsInfo
    #[serde(rename = "CharacterRoundsInfo")]
    pub character_rounds_info: CharacterRoundsInfo,
    /// stats_in_game
    #[serde(default)]
    pub stats_in_game: StatsInGame,
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
            equipment_on: HashMap::new(),
            attacks_list: IndexMap::new(),
            level: 1,
            color_theme: "dark".to_owned(),
            power: Powers::default(),
            character_rounds_info: CharacterRoundsInfo::default(),
            class: Class::Standard,
            stats_in_game: StatsInGame::default(),
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
        equipment_table: &HashMap<EquipmentJsonKey, Vec<Equipment>>,
    ) -> Result<Character> {
        if let Ok(mut value) = utils::read_from_json::<_, Character>(&path) {
            // init stats
            value.stats.init();
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
                // equipment loading
                let equipment_character_path = root_path
                    .as_ref()
                    .join(*OFFLINE_EQUIPMENT)
                    .join("characters")
                    .join(&value.db_full_name)
                    .with_extension("json");
                let Ok(decoded_equipment) =
                    Equipment::decode_characters_equipment(&equipment_character_path)
                else {
                    bail!(
                        "Equipment for character cannot be decoded: {:?}",
                        equipment_character_path.display()
                    );
                };
                value.equipment_on = decoded_equipment
                    .into_iter()
                    .map(|(k, v)| {
                        // Convert enum key to string for HashMap<String, Vec<Equipment>>
                        let key_string = k.to_string(); // <- requires Display impl on EquipmentJsonKey

                        // Turn EquipmentJsonValue into Vec<String>
                        let equipment_names: Vec<String> = match v {
                            EquipmentJsonValue::Single(name) => vec![name],
                            EquipmentJsonValue::Multiple(names) => names,
                        };

                        // Lookup the Equipment structs
                        let equipment_structs: Vec<Equipment> = equipment_names
                            .into_iter()
                            .filter_map(|name| {
                                equipment_table
                                    .get(&k) // still use enum key here
                                    .and_then(|equipments| {
                                        equipments.iter().find(|e| e.unique_name == name)
                                    })
                                    .cloned()
                                    .or_else(|| {
                                        if !name.is_empty() {
                                            tracing::error!(
                                                "Equipment {} cannot be found for character {}",
                                                name,
                                                value.db_full_name
                                            );
                                        }
                                        None
                                    })
                            })
                            .collect();

                        (key_string, equipment_structs)
                    })
                    .collect::<HashMap<String, Vec<Equipment>>>();
                // apply equipment on stats
                value.stats.apply_equipment_on_stats(
                    &value
                        .equipment_on
                        .values()
                        .flatten()
                        .cloned()
                        .collect::<Vec<Equipment>>(),
                );
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
        if ep.effect_type == EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE {
            self.stats
                .set_stats_on_effect(&ep.stats_name, -ep.value, true, true);
        }
        if ep.effect_type == EFFECT_IMPROVE_MAX_STAT_BY_VALUE {
            self.stats
                .set_stats_on_effect(&ep.stats_name, -ep.value, false, true);
        }
        self.character_rounds_info.remove_malus_effect(ep)?;
        Ok(())
    }

    pub fn remove_terminated_effect_on_player(&mut self) -> Result<Vec<EffectParam>> {
        let mut ended_effects: Vec<EffectParam> = Vec::new();
        for gae in self.character_rounds_info.all_effects.clone() {
            if gae.all_atk_effects.counter_turn == gae.all_atk_effects.input_effect_param.nb_turns {
                self.remove_malus_effect(&gae.all_atk_effects.input_effect_param)?;
                ended_effects.push(gae.all_atk_effects.input_effect_param.clone());
            }
        }
        self.character_rounds_info.all_effects.retain(|element| {
            element.all_atk_effects.input_effect_param.nb_turns
                != element.all_atk_effects.counter_turn
        });
        Ok(ended_effects)
    }

    pub fn reset_all_effects_on_player(&mut self) -> Result<()> {
        for gae in self.character_rounds_info.all_effects.clone() {
            self.remove_malus_effect(&gae.all_atk_effects.input_effect_param)?;
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

    pub fn apply_effect_outcome(
        &mut self,
        processed_ep: &ProcessedEffectParam,
        launcher_stats: &Stats,
        is_crit: bool,
        current_turn: usize, // to process aggro
    ) -> EffectOutcome {
        // eval if the effect can be applied on the target
        if processed_ep.input_effect_param.stats_name.is_empty()
            || !self
                .stats
                .all_stats
                .contains_key(&processed_ep.input_effect_param.stats_name)
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the stat {} does not exist.",
                processed_ep.input_effect_param.effect_type,
                self.id_name,
                processed_ep.input_effect_param.stats_name
            );
            return EffectOutcome {
                processed_effect_param: processed_ep.clone(),
                ..Default::default()
            };
        }

        // eval `full_amount`
        let mut full_amount;
        let mut processed_effect_param = processed_ep.clone();
        let pow_current =
            launcher_stats.get_power_stat(processed_ep.input_effect_param.is_magic_atk);
        if processed_ep.input_effect_param.stats_name == HP
            && processed_ep.input_effect_param.effect_type == EFFECT_NB_DECREASE_ON_TURN
        {
            // prepare for HOT
            full_amount = processed_ep.number_of_applies
                * (processed_ep.input_effect_param.value
                    + pow_current / processed_ep.input_effect_param.nb_turns);
            // update effect value
            processed_effect_param.input_effect_param.value = full_amount;
        } else if processed_ep.input_effect_param.stats_name == HP
            && processed_ep.input_effect_param.effect_type == EFFECT_VALUE_CHANGE
        {
            if processed_ep.input_effect_param.value > 0 {
                // HOT
                full_amount = processed_ep.number_of_applies
                    * (processed_ep.input_effect_param.value + pow_current)
                    / processed_ep.input_effect_param.nb_turns;
            } else {
                // DOT
                full_amount = processed_ep.number_of_applies
                    * AttackType::damage_by_atk(
                        &self.stats,
                        launcher_stats,
                        processed_ep.input_effect_param.is_magic_atk,
                        processed_ep.input_effect_param.value,
                        processed_ep.input_effect_param.nb_turns,
                    );
            }
        } else if processed_ep.input_effect_param.effect_type == EFFECT_PERCENT_CHANGE
            && Stats::is_energy_stat(&processed_ep.input_effect_param.stats_name)
        {
            full_amount = processed_ep.number_of_applies
                * self
                    .stats
                    .all_stats
                    .get(&processed_ep.input_effect_param.stats_name)
                    .unwrap()
                    .max as i64
                * processed_ep.input_effect_param.value
                / 100;
        } else {
            full_amount = processed_ep.number_of_applies * processed_ep.input_effect_param.value;
        }
        // Apply buf/debuf, crit, blocking on damages/heal
        if processed_ep.input_effect_param.stats_name == HP {
            full_amount = self.character_rounds_info.apply_buf_debuf(
                full_amount,
                &processed_ep.input_effect_param.target_kind,
                is_crit,
            );
            processed_effect_param.input_effect_param.value = full_amount;
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

        // Process non-stats `HP`
        // Otherwise update the max value of the stats
        if processed_ep.input_effect_param.stats_name != HP
            && (processed_ep.input_effect_param.effect_type == EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE
                || processed_ep.input_effect_param.effect_type == EFFECT_IMPROVE_MAX_STAT_BY_VALUE)
        {
            self.stats.set_stats_on_effect(
                &processed_ep.input_effect_param.stats_name,
                full_amount,
                processed_ep.input_effect_param.effect_type == EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE,
                true,
            );
        }
        // apply change current stats for non HP stats
        if processed_ep.input_effect_param.stats_name != HP
            && processed_ep.input_effect_param.effect_type == EFFECT_VALUE_CHANGE
        {
            let _ = self
                .stats
                .modify_stat_current(&processed_ep.input_effect_param.stats_name, full_amount);
        }

        // process aggro for `HP` and `non-HP` stats
        if processed_ep.input_effect_param.effect_type != EFFECT_IMPROVE_MAX_STAT_BY_VALUE
            && processed_ep.input_effect_param.effect_type != EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE
        {
            if processed_ep.input_effect_param.stats_name == HP {
                // process aggro for the launcher
                self.process_aggro(real_hp_amount, 0, current_turn);
            } else {
                // Add aggro to a target
                self.process_aggro(0, processed_ep.input_effect_param.value, current_turn);
            }
        }

        // update stats in game
        let eo = EffectOutcome {
            full_atk_amount_tx: full_amount,
            real_hp_amount_tx: real_hp_amount,
            processed_effect_param,
            target_kind: self.id_name.clone(),
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
    ) -> Result<Vec<ProcessedEffectParam>> {
        let mut processed_effect_param_list: Vec<ProcessedEffectParam> = vec![];
        for effect in atk.all_effects.clone() {
            processed_effect_param_list.push(
                self.character_rounds_info
                    .process_one_effect(&effect, atk, game_state, is_crit)?,
            );
        }
        Ok(processed_effect_param_list)
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
    }

    pub fn is_receiving_atk(
        &mut self,
        processed_ep: &ProcessedEffectParam,
        current_turn: usize,
        is_crit: bool,
        launcher_info: &LauncherAtkInfo,
    ) -> (Option<EffectOutcome>, Option<Vec<DodgeInfo>>) {
        let mut eo: Option<EffectOutcome> = None;
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
            eo = Some(self.apply_effect_outcome(
                processed_ep,
                &launcher_info.stats,
                is_crit,
                current_turn,
            ));
            // assess the blocking
            if self
                .character_rounds_info
                .is_blocking(&processed_ep.input_effect_param)
            {
                di.push(self.character_rounds_info.dodge_info.clone());
            }
            // update all effects
            self.character_rounds_info.all_effects.push(GameAtkEffects {
                all_atk_effects: processed_ep.clone(),
                atk: launcher_info.atk_type.clone(),
                launcher: launcher_info.id_name.clone(),
                target: "".to_owned(),
                launching_turn: current_turn,
            });
        } else {
            tracing::info!(
                "is_receiving_atk: effect is not applied on:{} current_turn:{}, kind:{:?}, launcher_info.id_name:{}, effect.target: {:?}, launcher_kind: {:?}, effect.type: {:?}, effect.stats_name: {}.",
                self.id_name,
                current_turn,
                self.kind,
                launcher_info.id_name,
                processed_ep.input_effect_param.target_kind,
                launcher_info.kind,
                processed_ep.input_effect_param.effect_type,
                processed_ep.input_effect_param.stats_name
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
        (eo, all_dodging)
    }

    /// The attak can be launched if the character has enough mana, vigor and
    /// berseck and if the atk is not under a cooldown.
    /// If the atk can be launched, true is returned, otherwise false is returned.
    pub fn can_be_launched(&self, atk_type: &AttackType) -> bool {
        // needed level too high
        if self.level < atk_type.level {
            return false;
        }

        // that attack has a cooldown
        for atk_effect in &atk_type.all_effects {
            if atk_effect.effect_type == EFFECT_NB_COOL_DOWN {
                for e in &self.character_rounds_info.all_effects {
                    if e.atk.name == atk_type.name
                        && e.all_atk_effects.input_effect_param.nb_turns
                            - e.all_atk_effects.counter_turn
                            > 0
                    {
                        return false;
                    }
                }
            }

            if atk_effect.stats_name == HP
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
                        message: format!("{} on {}", e.effect_type, e.stats_name),
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
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use strum::IntoEnumIterator;

    use super::Character;
    use crate::character_mod::attack_type::AttackType;
    use crate::character_mod::character::AmountType;
    use crate::character_mod::effect::EffectOutcome;
    use crate::character_mod::equipment::EquipmentJsonKey;
    use crate::common::constants::paths_const::TEST_OFFLINE_ROOT;
    use crate::testing::testing_all_characters::{self, testing_all_equipment, testing_character};
    use crate::{
        character_mod::buffers::BufTypes,
        character_mod::character::{CharacterKind, Class},
        character_mod::effect::EffectParam,
        common::constants::{all_target_const::TARGET_ALLY, effect_const::*, stats_const::*},
        server::players_manager::GameAtkEffects,
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
        assert_eq!(12, c.character_rounds_info.all_buffers.len());
        assert_eq!(3, c.character_rounds_info.all_buffers[0].buf_type);
        assert!(!c.character_rounds_info.all_buffers[0].is_passive_enabled);
        assert!(c.character_rounds_info.all_buffers[0].is_percent);
        assert_eq!(100, c.character_rounds_info.all_buffers[0].value);
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
        // powers
        assert!(!c.power.is_crit_heal_after_crit);
        assert!(c.power.is_damage_tx_heal_needy_ally);
        // stats
        // stats - aggro
        assert_eq!(0, c.stats.all_stats[AGGRO].current);
        assert_eq!(9999, c.stats.all_stats[AGGRO].max);
        // stats - aggro test init stats only on aggro
        assert_eq!(
            c.stats.all_stats[AGGRO].current_raw,
            c.stats.all_stats[AGGRO].current
        );
        assert_eq!(
            c.stats.all_stats[AGGRO].max_raw,
            c.stats.all_stats[AGGRO].max
        );
        // stats - aggro rate
        assert_eq!(1, c.stats.all_stats[AGGRO_RATE].current);
        assert_eq!(1, c.stats.all_stats[AGGRO_RATE].max);
        // stats - berseck
        assert_eq!(105, c.stats.all_stats[BERSERK].current);
        assert_eq!(210, c.stats.all_stats[BERSERK].max); // right ring + 10 to max berseck (ratio -> update current 100 -> 105)
        // stats - berseck_rate
        assert_eq!(5, c.stats.all_stats[BERSECK_RATE].current); // +4 right ring
        assert_eq!(5, c.stats.all_stats[BERSECK_RATE].max);
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
        assert_eq!(13, c.stats.all_stats[SPEED_REGEN].current);
        assert_eq!(13, c.stats.all_stats[SPEED_REGEN].max); // + 10% by amulet
        // stats - vigor
        assert_eq!(200, c.stats.all_stats[VIGOR].current);
        assert_eq!(200, c.stats.all_stats[VIGOR].max);
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
        assert_eq!(13, c.equipment_on.len());

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
            effect_type: EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE.to_string(),
            stats_name: HP.to_string(),
            value: -10,
            ..Default::default()
        };
        let result = c.remove_malus_effect(&ep);
        assert_eq!(148, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        let ep = EffectParam {
            effect_type: EFFECT_IMPROVE_MAX_STAT_BY_VALUE.to_string(),
            stats_name: HP.to_string(),
            value: -10,
            ..Default::default()
        };
        assert!(result.is_ok());

        let result = c.remove_malus_effect(&ep);
        assert!(result.is_ok());
        assert_eq!(158, c.stats.all_stats[HP].max);
        assert_eq!(1, c.stats.all_stats[HP].current);
        let ep = EffectParam {
            effect_type: EFFECT_BLOCK_HEAL_ATK.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };

        let result = c.remove_malus_effect(&ep);
        assert!(!c.character_rounds_info.is_heal_atk_blocked);
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };
        assert!(result.is_ok());

        let result = c.remove_malus_effect(&ep);
        assert!(result.is_ok());
        assert_eq!(
            -10,
            c.character_rounds_info.all_buffers[BufTypes::DamageTxPercent as usize].value
        );
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };

        let result = c.remove_malus_effect(&ep);
        assert!(result.is_ok());
        assert_eq!(
            -10,
            c.character_rounds_info.all_buffers[BufTypes::DamageRxPercent as usize].value
        );
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_HEAL_RX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };

        let result = c.remove_malus_effect(&ep);
        assert!(result.is_ok());
        assert_eq!(
            -10,
            c.character_rounds_info.all_buffers[BufTypes::HealRxPercent as usize].value
        );
        let ep = EffectParam {
            effect_type: EFFECT_CHANGE_HEAL_TX_BY_PERCENT.to_string(),
            stats_name: HP.to_string(),
            value: 10,
            ..Default::default()
        };

        let result = c.remove_malus_effect(&ep);
        assert!(result.is_ok());
        assert_eq!(
            -10,
            c.character_rounds_info.all_buffers[BufTypes::HealTxPercent as usize].value
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
            effect_type: EFFECT_NB_COOL_DOWN.to_string(),
            nb_turns: 10,
            target_kind: c.id_name.clone(),
            ..Default::default()
        };
        let atk = Default::default();
        let mut game_state = Default::default();
        // target is himself
        let processed_effect_param = c
            .character_rounds_info
            .process_one_effect(&ep, &atk, &game_state, false)
            .unwrap();
        assert_eq!(
            EFFECT_NB_COOL_DOWN,
            processed_effect_param.input_effect_param.effect_type
        );
        assert_eq!(10, processed_effect_param.input_effect_param.nb_turns);
        assert_eq!(
            c.id_name,
            processed_effect_param.input_effect_param.target_kind
        );
        assert_eq!(0, processed_effect_param.input_effect_param.value);
        assert_eq!(
            0,
            processed_effect_param.input_effect_param.sub_value_effect
        );
        assert_eq!(
            "Cooldown actif sur  de 10 tours.",
            processed_effect_param.log.message
        );

        // test - critical
        ep.stats_name = HP.to_owned();
        ep.effect_type = EFFECT_IMPROVE_MAX_STAT_BY_VALUE.to_owned();
        ep.value = 10;
        let processed_effect_param = c
            .character_rounds_info
            .process_one_effect(&ep, &atk, &game_state, true)
            .unwrap();
        assert_eq!(
            EFFECT_IMPROVE_MAX_STAT_BY_VALUE,
            processed_effect_param.input_effect_param.effect_type
        );
        assert_eq!(10, processed_effect_param.input_effect_param.nb_turns);
        assert_eq!(
            c.id_name,
            processed_effect_param.input_effect_param.target_kind
        );
        assert_eq!(15, processed_effect_param.input_effect_param.value);
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
        ep.effect_type = CONDITION_ENNEMIES_DIED.to_owned();
        ep.sub_value_effect = 10;
        ep.value = 0;
        let processed_effect_param = c
            .character_rounds_info
            .process_one_effect(&ep, &atk, &game_state, false)
            .unwrap();
        // focus on effect_type
        assert_eq!(
            EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE,
            processed_effect_param.input_effect_param.effect_type
        );
        assert_eq!(10, processed_effect_param.input_effect_param.nb_turns);
        assert_eq!(
            c.id_name,
            processed_effect_param.input_effect_param.target_kind
        );
        // focus on value
        assert_eq!(10, processed_effect_param.input_effect_param.value);
        assert_eq!(
            10,
            processed_effect_param.input_effect_param.sub_value_effect
        );
        assert_eq!(
            "Max stat of HP is up by 10%",
            processed_effect_param.log.message
        );
    }

    #[test]
    fn unit_remove_terminated_effect_on_player() {
        let mut c = testing_character();
        c.character_rounds_info
            .all_effects
            .push(GameAtkEffects::default());
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
        c.class = Class::Tank;
        c.process_dodging(atk_level);
        assert!(!c.character_rounds_info.dodge_info.is_dodging);
        assert!(c.character_rounds_info.dodge_info.is_blocking);
    }

    #[test]
    fn unit_process_critical_strike() {
        let mut c = testing_character();
        c.stats.all_stats[CRITICAL_STRIKE].current = 0;
        assert!(!c.process_critical_strike("atk1").unwrap());
        c.stats.all_stats[CRITICAL_STRIKE].current = 100;
        assert!(c.process_critical_strike("atk1").unwrap());
        assert!(
            !c.character_rounds_info.all_buffers[BufTypes::NextHealAtkIsCrit as usize]
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
        let eo = c.apply_effect_outcome(&processed_ep, &launcher_stats, false, 0);
        assert_eq!(
            eo,
            EffectOutcome {
                processed_effect_param: processed_ep,
                ..Default::default()
            }
        );

        // target is other ally
        processed_ep = build_hot_effect_individual();
        let old_hp = c2.stats.all_stats[HP].current;
        let eo = c2.apply_effect_outcome(&processed_ep, &launcher_stats, false, 0);
        assert_eq!(eo.full_atk_amount_tx, 20);
        assert_eq!(eo.real_hp_amount_tx, 20);
        assert_eq!(eo.processed_effect_param.input_effect_param.value, 20);
        assert_eq!(old_hp + 20, c2.stats.all_stats[HP].current);
        assert_eq!(
            eo.processed_effect_param.input_effect_param.effect_type,
            EFFECT_VALUE_CHANGE
        );
        assert_eq!(eo.processed_effect_param.input_effect_param.stats_name, HP);
        assert_eq!(eo.processed_effect_param.input_effect_param.nb_turns, 2);
        assert_eq!(eo.processed_effect_param.number_of_applies, 1);
        assert!(!eo.processed_effect_param.input_effect_param.is_magic_atk);
        assert_eq!(
            eo.processed_effect_param.input_effect_param.target_kind,
            TARGET_ALLY
        );

        // target is ennemy
        let mut boss1 = Character::try_new_from_json(
            "./tests/offlines/characters/test_boss1.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        processed_ep = build_dmg_effect_individual();
        let old_hp = boss1.stats.all_stats[HP].current;
        let eo = boss1.apply_effect_outcome(&processed_ep, &launcher_stats, false, 0);
        assert_eq!(eo.full_atk_amount_tx, -40);
        assert_eq!(eo.real_hp_amount_tx, -40);
        assert_eq!(eo.processed_effect_param.input_effect_param.value, -40);
        assert_eq!(old_hp - 40, boss1.stats.all_stats[HP].current);

        processed_ep = build_buf_effect_individual_speed_regen();
        let launcher_stats = c.stats.clone();
        let eo = c.apply_effect_outcome(&processed_ep, &launcher_stats, false, 0);
        assert_eq!(eo.full_atk_amount_tx, 60);
        assert_eq!(eo.real_hp_amount_tx, 0);
        assert_eq!(
            eo.processed_effect_param.input_effect_param.stats_name,
            SPEED_REGEN
        );
        assert_eq!(eo.processed_effect_param.input_effect_param.value, 10);
    }

    #[test]
    fn unit_proces_aggro() {
        let mut c = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        c.init_aggro_on_turn(0);
        c.process_aggro(0, 0, 0);
        assert_eq!(
            0,
            c.character_rounds_info.tx_rx[AmountType::Aggro as usize][&0]
        );

        c.process_aggro(20, 0, 0);
        assert_eq!(
            1,
            c.character_rounds_info.tx_rx[AmountType::Aggro as usize][&0]
        );
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
        c.character_rounds_info.all_effects.push(GameAtkEffects {
            all_atk_effects: build_effect_max_stats(),
            atk: AttackType::default(),
            launcher: "".to_owned(),
            target: "".to_owned(),
            launching_turn: 0,
        });
        let effect_value = c.character_rounds_info.all_effects[0]
            .all_atk_effects
            .input_effect_param
            .value;
        c.reset_all_effects_on_player().unwrap();
        assert_eq!(
            hp_without_malus - effect_value,
            c.stats.all_stats[HP].max as i64
        );
        assert!(c.character_rounds_info.all_effects.is_empty());
    }

    #[test]
    fn unit_apply_equipment_on_stats() {
        let c = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        assert_eq!(12 + 10 * 12 / 100, c.stats.all_stats[SPEED_REGEN].current);
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
        c1.character_rounds_info.all_effects.push(GameAtkEffects {
            all_atk_effects: build_cooldown_effect(),
            atk: atk_type.clone(),
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
        processed_ep.counter_turn = processed_ep.input_effect_param.nb_turns;
        c1.character_rounds_info.all_effects.clear();
        c1.character_rounds_info.all_effects.push(GameAtkEffects {
            all_atk_effects: processed_ep.clone(),
            atk: atk_type.clone(),
            ..Default::default()
        });
        let result = c1.can_be_launched(&atk_type);
        assert!(result);
        // not enough berseck
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
}
