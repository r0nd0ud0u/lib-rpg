use anyhow::{Result, anyhow, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, vec};

use crate::{
    attack_type::{AttackType, LauncherAtkInfo},
    buffers::BufTypes,
    character_mod::rounds_information::CharacterRoundsInfo,
    common::{
        all_target_const::*, attak_const::COEFF_CRIT_STATS, character_const::ULTIMATE_LEVEL,
        effect_const::*, paths_const::*, reach_const::*, stats_const::*,
    },
    effect::{
        EffectOutcome, EffectParam, ProcessedEffectParam, is_boosted_by_crit,
        process_decrease_on_turn,
    },
    equipment::{Equipment, EquipmentJsonKey, EquipmentJsonValue},
    game_manager::LogData,
    game_state::GameState,
    players_manager::{DodgeInfo, GameAtkEffects},
    powers::Powers,
    stats::Stats,
    stats_in_game::StatsInGame,
    target::is_target_ally,
    utils::{self, get_random_nb, list_files_in_dir},
};

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
    pub kind: CharacterType,
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
    /// TODO rank
    #[serde(rename = "Rank")]
    pub rank: u64,
    /// TODO shape
    #[serde(rename = "Shape")]
    pub shape: String,
    #[serde(default, rename = "Effects")]
    pub all_effects: Vec<GameAtkEffects>,
    /// Fight information: stats_in_game
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
            kind: CharacterType::Hero,
            equipment_on: HashMap::new(),
            attacks_list: IndexMap::new(),
            level: 1,
            color_theme: "dark".to_owned(),
            power: Powers::default(),
            character_rounds_info: CharacterRoundsInfo::default(),
            class: Class::Standard,
            rank: 0,
            shape: String::new(),
            all_effects: vec![],
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
    // TODO add function to validate json
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

    // TODO if I remove a malus percent for DamageTx with EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT, how can if make the difference with a value which is not percent
    pub fn remove_malus_effect(&mut self, ep: &EffectParam) -> Result<()> {
        if ep.effect_type == EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE {
            self.stats
                .set_stats_on_effect(&ep.stats_name, -ep.value, true, true);
        }
        if ep.effect_type == EFFECT_IMPROVE_MAX_STAT_BY_VALUE {
            self.stats
                .set_stats_on_effect(&ep.stats_name, -ep.value, false, true);
        }
        if ep.effect_type == EFFECT_BLOCK_HEAL_ATK {
            self.character_rounds_info.is_heal_atk_blocked = false;
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

    pub fn update_buf(
        &mut self,
        buf_type: &BufTypes,
        value: i64,
        is_percent: bool,
        stat: &str,
    ) -> Result<()> {
        if let Some(buf) = self
            .character_rounds_info
            .all_buffers
            .get_mut(buf_type.clone() as usize)
        {
            buf.update_buf(value, is_percent, stat);
            Ok(())
        } else {
            bail!("Buffer type {:?} cannot be found", buf_type);
        }
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
        let bug_apply_init =
            &self.character_rounds_info.all_buffers[BufTypes::ApplyEffectInit as usize];
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

    pub fn increment_counter_effect(&mut self) {
        for gae in self.all_effects.iter_mut() {
            gae.all_atk_effects.counter_turn += 1;
        }
    }

    pub fn remove_terminated_effect_on_player(&mut self) -> Result<Vec<EffectParam>> {
        let mut ended_effects: Vec<EffectParam> = Vec::new();
        for gae in self.all_effects.clone() {
            if gae.all_atk_effects.counter_turn == gae.all_atk_effects.input_effect_param.nb_turns {
                self.remove_malus_effect(&gae.all_atk_effects.input_effect_param)?;
                ended_effects.push(gae.all_atk_effects.input_effect_param.clone());
            }
        }
        self.all_effects.retain(|element| {
            element.all_atk_effects.input_effect_param.nb_turns
                != element.all_atk_effects.counter_turn
        });
        Ok(ended_effects)
    }

    pub fn reset_all_effects_on_player(&mut self) -> Result<()> {
        for gae in self.all_effects.clone() {
            self.remove_malus_effect(&gae.all_atk_effects.input_effect_param)?;
        }
        self.all_effects.clear();
        Ok(())
    }

    pub fn reset_all_buffers(&mut self) {
        self.character_rounds_info
            .all_buffers
            .iter_mut()
            .for_each(|b| {
                b.set_buffers(0, false);
                b.is_passive_enabled = false;
            });
    }

    pub fn process_atk_cost(&mut self, atk_name: &str) {
        if let Some(atk) = self.attacks_list.get(atk_name) {
            if let Some(mana) = self.stats.all_stats.get_mut(MANA) {
                mana.current = std::cmp::max(
                    0,
                    mana.current
                        .saturating_sub(atk.mana_cost.saturating_mul(mana.max) / 100),
                );
            }
            if let Some(vigor) = self.stats.all_stats.get_mut(VIGOR) {
                vigor.current = std::cmp::max(
                    0,
                    vigor
                        .current
                        .saturating_sub(atk.vigor_cost.saturating_mul(vigor.max) / 100),
                );
            }
            if let Some(berseck) = self.stats.all_stats.get_mut(BERSERK) {
                berseck.current = std::cmp::max(
                    0,
                    berseck
                        .current
                        .saturating_sub(atk.berseck_cost.saturating_mul(berseck.max) / 100),
                );
            }
        }
    }

    pub fn process_dodging(&mut self, atk_level: u64) {
        let dodge_info = if atk_level == ULTIMATE_LEVEL {
            DodgeInfo {
                name: self.id_name.clone(),
                is_dodging: false,
                is_blocking: false,
            }
        } else {
            let rand_nb = get_random_nb(1, 100);
            let is_dodging =
                self.class != Class::Tank && rand_nb <= self.stats.all_stats[DODGE].current as i64;
            let is_blocking = self.class == Class::Tank;
            DodgeInfo {
                name: self.id_name.clone(),
                is_dodging,
                is_blocking,
            }
        };
        self.character_rounds_info.dodge_info = dodge_info;
    }

    pub fn process_critical_strike(&mut self, atk_name: &str) -> Result<bool> {
        let atk = if let Some(atk) = self.attacks_list.get(atk_name) {
            atk
        } else {
            return Ok(false);
        };
        // process passive power
        let is_crit_by_passive = self.character_rounds_info.all_buffers
            [BufTypes::NextHealAtkIsCrit as usize]
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
                self.update_buf(&BufTypes::DamageCritCapped, delta_capped, false, "")?;
            }
            Ok(true)
        } else if is_crit_by_passive {
            self.character_rounds_info.all_buffers[BufTypes::NextHealAtkIsCrit as usize]
                .is_passive_enabled = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn is_targeted(
        &self,
        effect: &EffectParam,
        launcher_id_name: &str,
        launcher_kind: &CharacterType,
    ) -> bool {
        let is_ally = self.kind == *launcher_kind;
        if effect.target_kind == TARGET_HIMSELF && launcher_id_name != self.id_name {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is himself.",
                effect.effect_type,
                self.id_name
            );
            return false;
        }
        if effect.target_kind == TARGET_ONLY_ALLY && launcher_id_name == self.id_name {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is only ally but launcher is himself.",
                effect.effect_type,
                self.id_name
            );
            return false;
        }
        if !is_ally && is_target_ally(&effect.target_kind) {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is ally but launcher is ennemy.",
                effect.effect_type,
                self.id_name
            );
            return false;
        }
        if is_ally && effect.target_kind == TARGET_ENNEMY {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is ennemy but launcher is ally.",
                effect.effect_type,
                self.id_name
            );
            return false;
        }
        // is targeted ?
        if effect.target_kind == TARGET_ALLY
            && effect.reach == INDIVIDUAL
            && !self.character_rounds_info.is_current_target
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is ally but not current target.",
                effect.effect_type,
                self.id_name
            );
            return false;
        }
        if effect.target_kind == TARGET_ENNEMY
            && effect.reach == INDIVIDUAL
            && !self.character_rounds_info.is_current_target
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is ennemy but not current target.",
                effect.effect_type,
                self.id_name
            );
            return false;
        }
        if effect.target_kind == TARGET_ALLY
            && effect.reach == ZONE
            && launcher_id_name == self.id_name
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is ally but launcher is himself.",
                effect.effect_type,
                self.id_name
            );
            return false;
        }
        if self.character_rounds_info.is_dodging(&effect.target_kind)
            && self.kind != *launcher_kind
            && self.character_rounds_info.is_current_target
        {
            tracing::debug!(
                "Effect {} cannot be applied on {} because the target is dodging.",
                effect.effect_type,
                self.id_name
            );
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
        processed_ep: &ProcessedEffectParam,
        launcher_stats: &Stats,
        is_crit: bool,
        current_turn: usize, // to process aggro
    ) -> EffectOutcome {
        if processed_ep.input_effect_param.stats_name.is_empty()
            || !self
                .stats
                .all_stats
                .contains_key(&processed_ep.input_effect_param.stats_name)
        {
            tracing::info!(
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
        // Return now if the full amount is 0
        if full_amount == 0 {
            tracing::info!(
                "Effect {} has no impact on {} because the full amount is 0.",
                processed_ep.input_effect_param.effect_type,
                self.id_name
            );
            return EffectOutcome::default();
        }

        // apply buf/debuf to full_amount in case of damages/heal
        if processed_ep.input_effect_param.stats_name == HP {
            full_amount = self.character_rounds_info.apply_buf_debuf(
                full_amount,
                &processed_ep.input_effect_param.target_kind,
                is_crit,
            );
            processed_effect_param.input_effect_param.value = full_amount;
        }

        // Otherwise update the current value of the stats or the HOT/DOT
        // stats update
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
            tracing::info!(
                "Effect {} applied on {} for stat {} by {}{}.",
                processed_ep.input_effect_param.effect_type,
                self.id_name,
                processed_ep.input_effect_param.stats_name,
                full_amount,
                if processed_ep.input_effect_param.effect_type
                    == EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE
                {
                    "%"
                } else {
                    ""
                }
            );
            return EffectOutcome {
                full_atk_amount_tx: full_amount,
                real_hp_amount_tx: full_amount,
                processed_effect_param,
                target_kind: self.id_name.clone(),
                ..Default::default()
            };
        }
        if processed_ep.input_effect_param.stats_name != HP
            && processed_ep.input_effect_param.effect_type == EFFECT_VALUE_CHANGE
        {
            self.stats
                .modify_stat_current(&processed_ep.input_effect_param.stats_name, full_amount);
        }
        // blocking the atk
        if self
            .character_rounds_info
            .is_blocking(&processed_ep.input_effect_param)
        {
            full_amount = 10 * full_amount / 100;
        }
        // Calculation of the real amount of the value of the effect and update the energy stats
        let real_hp_amount =
            self.update_hp_process_real_amount(&processed_ep.input_effect_param, full_amount);

        // process aggro
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
            processed_effect_param_list
                .push(self.process_one_effect(&effect, atk, game_state, is_crit)?);
        }
        Ok(processed_effect_param_list)
    }

    /// access the real amount received by the effect on that character
    pub fn update_hp_process_real_amount(&mut self, ep: &EffectParam, full_amount: i64) -> i64 {
        if ep.stats_name != HP {
            return 0;
        }
        let real_hp_amount;
        if full_amount > 0 {
            // heal
            let delta =
                self.stats.all_stats[HP].max as i64 - self.stats.all_stats[HP].current as i64;
            self.stats.all_stats[HP].current = std::cmp::min(
                full_amount + self.stats.all_stats[HP].current as i64,
                self.stats.all_stats[HP].max as i64,
            ) as u64;
            real_hp_amount = std::cmp::min(delta, full_amount);
        } else {
            // damage
            let tmp = self.stats.all_stats[HP].current as i64;
            self.stats.all_stats[HP].current =
                std::cmp::max(0, self.stats.all_stats[HP].current as i64 + full_amount) as u64;
            real_hp_amount = std::cmp::max(-tmp, full_amount);
        }
        real_hp_amount
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
        // check if the effect is applied on the target
        if self.is_targeted(
            &processed_ep.input_effect_param,
            &launcher_info.id_name,
            &launcher_info.kind,
        ) {
            // TODO check if the effect is not already applied
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
            self.all_effects.push(GameAtkEffects {
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
                for e in &self.all_effects {
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
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use strum::IntoEnumIterator;

    use super::Character;
    use crate::attack_type::AttackType;
    use crate::buffers::Buffers;
    use crate::character::AmountType;
    use crate::common::paths_const::TEST_OFFLINE_ROOT;
    use crate::effect::EffectOutcome;
    use crate::equipment::EquipmentJsonKey;
    use crate::testing_all_characters::{testing_all_equipment, testing_character};
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
        // TODO change
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
        // rank
        assert_eq!(4, c.rank);
        // shape
        assert_eq!("", c.shape);
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
        assert_eq!(CharacterType::Hero, c.kind);
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
            c.character_rounds_info.all_buffers[BufTypes::DamageTx as usize].value
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
            c.character_rounds_info.all_buffers[BufTypes::DamageRx as usize].value
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
            c.character_rounds_info.all_buffers[BufTypes::HealRx as usize].value
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
            c.character_rounds_info.all_buffers[BufTypes::HealTx as usize].value
        );
    }

    #[test]
    fn unit_update_buf() {
        let file_path = "./tests/offlines/characters/test.json"; // Path to the JSON file
        let c = Character::try_new_from_json(
            file_path,
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        );
        assert!(c.is_ok());
        let mut c = c.unwrap();
        let result = c.update_buf(&BufTypes::DamageTx, 10, false, HP);
        assert_eq!(
            10,
            c.character_rounds_info.all_buffers[BufTypes::DamageTx as usize].value
        );
        assert!(result.is_ok());
        assert!(!c.character_rounds_info.all_buffers[BufTypes::DamageTx as usize].is_percent);
        assert_eq!(
            HP,
            c.character_rounds_info.all_buffers[BufTypes::DamageTx as usize].all_stats_name[0]
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
        let processed_effect_param = c.process_one_effect(&ep, &atk, &game_state, false).unwrap();
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
        let processed_effect_param = c.process_one_effect(&ep, &atk, &game_state, true).unwrap();
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
        let processed_effect_param = c.process_one_effect(&ep, &atk, &game_state, false).unwrap();
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
        c.all_effects.push(GameAtkEffects::default());
        c.remove_terminated_effect_on_player().unwrap();
        assert_eq!(0, c.all_effects.len());
        // TODO improve the test  by checking if the effect is removed on character stats
    }

    #[test]
    fn unit_process_atk_cost() {
        let mut c = testing_character();
        let old_vigor = c.stats.all_stats[VIGOR].current;
        c.process_atk_cost("atk1"); // 10% vigor cost
        assert_eq!(old_vigor - 20, c.stats.all_stats[VIGOR].current);
        c.process_atk_cost("atk1"); // 10% vigor cost again!
        assert_eq!(old_vigor - 40, c.stats.all_stats[VIGOR].current);
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
    fn unit_is_targeted() {
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
        let mut ep = build_cooldown_effect().input_effect_param;
        // target is himself
        assert!(c1.is_targeted(&ep, &c1.id_name, &c1.kind));
        // other ally
        assert!(!c2.is_targeted(&ep, &c1.id_name, &c1.kind));
        // boss
        assert!(!boss1.is_targeted(&ep, &c1.id_name, &c1.kind));

        // effect on ally individual
        ep = build_hot_effect_individual().input_effect_param;
        // target is himself
        assert!(!c1.is_targeted(&ep, &c1.id_name, &c1.kind));
        // other ally
        // not targeted on main atk
        c2.character_rounds_info.is_current_target = false;
        assert!(!c2.is_targeted(&ep, &c1.id_name, &c1.kind));
        // targeted on main atk
        c2.character_rounds_info.is_current_target = true;
        assert!(c2.is_targeted(&ep, &c1.id_name, &c1.kind));
        // boss
        assert!(!boss1.is_targeted(&ep, &c1.id_name, &c1.kind));

        // effect on ennemy individual
        ep = build_dmg_effect_individual().input_effect_param;
        assert!(!c1.is_targeted(&ep, &c1.id_name, &c1.kind));
        // other ally
        assert!(!c2.is_targeted(&ep, &c1.id_name, &c1.kind));
        // boss
        // targeted on main atk
        boss1.character_rounds_info.is_current_target = true;
        assert!(boss1.is_targeted(&ep, &c1.id_name, &c1.kind));
        // not targeted on main atk
        boss1.character_rounds_info.is_current_target = false;
        assert!(!boss1.is_targeted(&ep, &c1.id_name, &c1.kind));

        // effect on ally ZONE
        ep = build_hot_effect_zone().input_effect_param;
        // target is himself
        assert!(!c1.is_targeted(&ep, &c1.id_name, &c1.kind));
        // other ally
        // targeted on main atk
        assert!(c2.is_targeted(&ep, &c1.id_name, &c1.kind));
        // boss
        assert!(!boss1.is_targeted(&ep, &c1.id_name, &c1.kind));

        // effect on ennemy ZONE
        ep = build_dot_effect_zone().input_effect_param;
        // target is himself
        assert!(!c1.is_targeted(&ep, &c1.id_name, &c1.kind));
        // other ally
        assert!(!c2.is_targeted(&ep, &c1.id_name, &c1.kind));
        // boss
        // targeted on main atk
        boss1.character_rounds_info.is_current_target = true;
        assert!(boss1.is_targeted(&ep, &c1.id_name, &c1.kind));
        // not targeted on main atk
        boss1.character_rounds_info.is_current_target = false;
        assert!(boss1.is_targeted(&ep, &c1.id_name, &c1.kind));

        // effect on all allies
        ep = build_hot_effect_all().input_effect_param;
        // target is himself
        assert!(c1.is_targeted(&ep, &c1.id_name, &c1.kind));
        assert!(c1.is_targeted(&ep, &c1.id_name, &c1.kind));
        // other ally
        assert!(c2.is_targeted(&ep, &c1.id_name, &c1.kind));
        assert!(c2.is_targeted(&ep, &c1.id_name, &c1.kind));
        // boss
        // targeted on main atk
        boss1.character_rounds_info.is_current_target = true;
        assert!(!boss1.is_targeted(&ep, &c1.id_name, &c1.kind));
        boss1.character_rounds_info.is_current_target = false;
        assert!(!boss1.is_targeted(&ep, &c1.id_name, &c1.kind));
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
        assert_eq!(eo.real_hp_amount_tx, 60);
        assert_eq!(
            eo.processed_effect_param.input_effect_param.stats_name,
            SPEED_REGEN
        );
        assert_eq!(eo.processed_effect_param.input_effect_param.value, 10);
    }

    #[test]
    fn unit_process_real_amount() {
        let mut c = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        let old_hp = c.stats.all_stats[HP].current;
        let result = c.update_hp_process_real_amount(
            &build_dmg_effect_individual().input_effect_param,
            -(c.stats.all_stats[HP].current as i64) - 10,
        );
        // real amount cannot excess the life of the character
        assert_eq!(result, -(old_hp as i64));
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
        c.all_effects.push(GameAtkEffects {
            all_atk_effects: build_effect_max_stats(),
            atk: AttackType::default(),
            launcher: "".to_owned(),
            target: "".to_owned(),
            launching_turn: 0,
        });
        let effect_value = c.all_effects[0].all_atk_effects.input_effect_param.value;
        c.reset_all_effects_on_player().unwrap();
        assert_eq!(
            hp_without_malus - effect_value,
            c.stats.all_stats[HP].max as i64
        );
        assert!(c.all_effects.is_empty());
    }

    #[test]
    fn unit_reset_all_buffers() {
        let mut c = Character::try_new_from_json(
            "./tests/offlines/characters/test.json",
            *TEST_OFFLINE_ROOT,
            false,
            &testing_all_equipment(),
        )
        .unwrap();
        let mut b = Buffers::default();
        b.set_buffers(30, true);
        c.character_rounds_info.all_buffers.push(b);
        c.reset_all_buffers();
        assert_eq!(c.character_rounds_info.all_buffers[0].value, 0);
        assert!(!c.character_rounds_info.all_buffers[0].is_percent);
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
        c1.all_effects.push(GameAtkEffects {
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
        c1.all_effects.clear();
        c1.all_effects.push(GameAtkEffects {
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
        c1.all_effects.clear();
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
}
