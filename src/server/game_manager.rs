use std::{collections::HashMap, path::Path};

use crate::{
    character_mod::{
        attack_type::{AttackType, LauncherAtkInfo},
        buffers::BufKinds,
        character::{Character, CharacterKind},
        class::Class,
        equipment::{Equipment, EquipmentJsonKey},
        experience::{build_exp_to_next_level, build_experience},
        loot::LootType,
        rank::Rank,
        rounds_information::AmountType,
    },
    common::{
        constants::{character_const::ULTIMATE_LEVEL, paths_const::*, stats_const::*},
        log_data::{
            LogData,
            const_colors::{DARK_RED, LIGHT_BLUE, LIGHT_GREEN},
        },
    },
    server::{
        game_paths::GamePaths,
        game_state::{GameState, GameStatus},
        players_manager::{DodgeInfo, GameAtkEffect, PlayerManager},
        scenario::{Scenario, ScenarioState},
    },
    utils,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultLaunchAttack {
    pub launcher_id_name: String,
    pub new_game_atk_effects: Vec<GameAtkEffect>,
    pub is_crit: bool,
    pub all_dodging: Vec<DodgeInfo>,
    pub is_boss_atk: bool,
    pub logs_end_of_round: Vec<LogData>,
    pub logs_atk: Vec<LogData>,
    pub turn_nb: usize,
    pub round_nb: usize,
}

/// The entry of the library.
/// That object should be called to access to all the different functionalities.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManager {
    /// Game state of the current game
    pub game_state: GameState,
    /// Player manager
    pub pm: PlayerManager,
    /// Paths of the current game
    pub game_paths: GamePaths,
    /// logs of the game, to display in the log sheet
    pub logs: Vec<LogData>,
    /// Current scenario of the game, to adapt the behavior of the fight
    pub current_scenario: Scenario,
    /// all scenarios
    pub all_scenarios: Vec<Scenario>,
    /// State of the different scenarios, to know which scenario is available for the player and to adapt the behavior of the fight
    pub states_scenarios: HashMap<String, ScenarioState>,
}

impl GameManager {
    /// Create a new game manager with the given path for the offline files and the default active characters
    pub fn new<P: AsRef<Path>>(
        path: P,
        equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>,
        scenarios: Vec<Scenario>,
    ) -> GameManager {
        // if path is empty, use the default one
        let mut new_path = path.as_ref();
        if new_path.as_os_str().is_empty() {
            new_path = &OFFLINE_ROOT;
        }
        // create game state
        let game_state = GameState::new();
        let game_name = game_state.game_name.clone();

        // scenarios state
        let mut states_scenarios = HashMap::new();
        for scenario in &scenarios {
            states_scenarios.insert(scenario.name.clone(), ScenarioState::NotStarted);
        }

        GameManager {
            game_state,
            pm: PlayerManager::new(equipment_table),
            game_paths: GamePaths::new(new_path, &game_name),
            logs: Vec::new(),
            current_scenario: Scenario::default(),
            all_scenarios: scenarios,

            states_scenarios,
        }
    }

    /// Set active bosses from the current scenario's boss patterns.
    /// Bosses whose name matches a pattern in the current scenario are cloned and
    /// pushed into `pm.active_bosses` with a unique id_name (`"<name>_#<n>"`).
    pub fn set_active_bosses(&mut self, all_bosses: &[Character]) {
        self.current_scenario
            .boss_patterns
            .iter()
            .for_each(|(boss_name, _)| {
                if let Some(b) = all_bosses.iter().find(|b| b.db_full_name == *boss_name) {
                    let mut boss_to_push = b.clone();
                    boss_to_push.id_name = format!(
                        "{}_#{}",
                        boss_to_push.db_full_name,
                        1 + self
                            .pm
                            .get_nb_of_active_bosses_by_name(&boss_to_push.db_full_name)
                    );
                    self.pm.active_bosses.push(boss_to_push);
                } else {
                    tracing::warn!("Boss {} not found in data manager, skipping it", boss_name);
                }
            });
    }

    pub fn load_next_scenario(&mut self) -> Result<()> {
        // update current scenario state
        if let Some((_, state)) = self
            .states_scenarios
            .iter_mut()
            .find(|(name, _)| *name == &self.current_scenario.name)
        {
            *state = ScenarioState::Completed;
        }
        let current_level = self.current_scenario.level;
        // get the next scenario with the next level
        let Some(scenario) = self
            .all_scenarios
            .iter()
            .find(|s| s.level == current_level + 1)
            .cloned()
        else {
            return Err(anyhow::anyhow!(
                "No next scenario found for level {}",
                current_level + 1
            ));
        };
        // update scenario state in map
        if let Some((_, state)) = self
            .states_scenarios
            .iter_mut()
            .find(|(name, _)| *name == &scenario.name)
        {
            *state = ScenarioState::InProgress;
        }
        // update current scenario
        self.current_scenario = scenario;
        // clear previous scenario
        self.game_state.clear_scenario();
        self.pm.clear_scenario();
        // set active bosses for the new scenario from the stored roster
        let all_bosses = self.pm.all_bosses.clone();
        self.set_active_bosses(&all_bosses);
        let _ = self.start_new_turn();

        Ok(())
    }

    pub fn all_scenarios_completed(&self) -> bool {
        self.states_scenarios
            .values()
            .all(|state| *state == ScenarioState::Completed)
    }

    /// Start the game by starting a new turn
    pub fn start_game(&mut self) {
        // Start a new turn
        let _ = self.start_new_turn();
    }

    /// Process the start of a new turn:
    /// - Process the order of the players to play
    /// - Increment the turn number
    /// - Reset the round number
    ///
    /// Return a boolean to know if the new turn has been started and the logs of the new round if it is the case
    pub fn start_new_turn(&mut self) -> (bool, Vec<LogData>) {
        // For each turn now
        // Process the order of the players
        self.process_order_to_play();
        self.game_state.start_new_turn();
        self.pm.start_new_turn();

        self.new_round()
    }

    /// Process the order of the players to play by sorting them by speed and adding the supplementary atk turns for the heroes and the bosses
    pub fn process_order_to_play(&mut self) {
        // to be improved with stats
        // one player can play several times as well in different order
        self.game_state.order_to_play.clear();

        // add heroes
        // sort by speed
        self.pm
            .active_heroes
            .sort_by(|a, b| a.stats.all_stats[SPEED].cmp(&b.stats.all_stats[SPEED]));
        let mut dead_heroes = Vec::new();
        for hero in &self.pm.active_heroes {
            if !hero.stats.is_dead().unwrap_or(false) {
                self.game_state.order_to_play.push(hero.id_name.clone());
            } else {
                dead_heroes.push(hero.id_name.clone());
            }
        }
        // add dead heroes
        for name in dead_heroes {
            self.game_state.order_to_play.push(name);
        }
        // add bosses
        // sort by speed
        self.pm
            .active_bosses
            .sort_by(|a, b| a.stats.all_stats[SPEED].cmp(&b.stats.all_stats[SPEED]));
        for boss in &self.pm.active_bosses {
            if !boss.stats.is_dead().unwrap_or(false) {
                self.game_state.order_to_play.push(boss.id_name.clone());
            }
        }
        // supplementary atks to be added
        let supp_rounds_heroes = self.pm.process_sup_atk_turn(CharacterKind::Hero);
        let supp_rounds_bosses = self.pm.process_sup_atk_turn(CharacterKind::Boss);
        self.game_state.order_to_play.extend(supp_rounds_heroes);
        self.game_state.order_to_play.extend(supp_rounds_bosses);
    }

    pub fn new_round(&mut self) -> (bool, Vec<LogData>) {
        self.game_state.new_round();
        // Still round to play
        if self.game_state.current_round > self.game_state.order_to_play.len() {
            return (
                false,
                vec![LogData {
                    message: "End of turn has been reached".to_string(),
                    ..Default::default()
                }],
            );
        }
        let Ok(logs) = self.pm.update_current_player_on_new_round(
            &self.game_state,
            &self.game_state.order_to_play[self.game_state.current_round - 1],
        ) else {
            // return the error of update_current_player
            return (
                false,
                vec![LogData {
                    message: "Error while updating current player".to_string(),
                    ..Default::default()
                }],
            );
        };

        if self.pm.current_player.stats.is_dead() == Some(true) {
            return self.new_round();
        }

        self.pm.reset_targeted_character();

        (true, logs)
    }

    /// Launch an attack from the current player
    /// If atk_name is None and it is an auto round (boss), a random atk will be chosen
    /// Otherwise, if atk_name is None, no atk will be launched
    pub fn launch_attack(&mut self, atk_name: Option<&str>) -> ResultLaunchAttack {
        // is atk existing?
        let Some(atk_name) = atk_name else {
            if self.is_round_auto() {
                // check if pattern exists in scenario
                if let Some(patterns) = self
                    .current_scenario
                    .boss_patterns
                    .get(&self.pm.current_player.id_name)
                    .cloned()
                {
                    // fill queue from pattern on first use, then cycle
                    if self
                        .pm
                        .current_player
                        .character_rounds_info
                        .atk_pattern_queue
                        .is_empty()
                    {
                        self.pm
                            .current_player
                            .character_rounds_info
                            .atk_pattern_queue
                            .extend(patterns.iter().copied());
                    }
                    if let Some(idx) = self
                        .pm
                        .current_player
                        .character_rounds_info
                        .atk_pattern_queue
                        .pop_front()
                        && let Some((atk_name, _)) =
                            self.pm.current_player.attacks_list.get_index(idx as usize)
                    {
                        let atk_name = atk_name.clone();
                        tracing::info!(
                            "Auto attack for boss {}: {}",
                            self.pm.current_player.id_name,
                            atk_name
                        );
                        return self.launch_attack(Some(&atk_name));
                    }
                }
                // auto atk for boss
                if let Some(auto_atk_name) = AttackType::get_one_random_atk_name(
                    &self.pm.current_player.character_rounds_info.launchable_atks,
                ) {
                    tracing::info!(
                        "Auto attack for boss {}: {}",
                        self.pm.current_player.id_name,
                        auto_atk_name
                    );
                    return self.launch_attack(Some(&auto_atk_name));
                }
            }

            return self.process_no_atk_launched();
        };
        // output
        let mut new_game_atk_effects: Vec<GameAtkEffect> = vec![];
        // update action done in round
        self.pm
            .current_player
            .character_rounds_info
            .actions_done_in_round += 1;
        // get all players
        let all_players = self.pm.get_all_active_id_names();
        // get atk
        let atk_list = self.pm.current_player.attacks_list.clone();
        let atk = match atk_list.get(atk_name) {
            Some(atk) => atk.clone(),
            None => {
                // unknown atk
                tracing::error!(
                    "Error: attack {} not found for player {}",
                    atk_name,
                    self.pm.current_player.id_name
                );
                return self.process_no_atk_launched();
            }
        };

        // can be launched
        // process cost
        self.pm.current_player.process_atk_cost(atk_name);

        // is dodging ?
        self.pm.process_all_dodging(
            &all_players,
            self.pm.current_player.attacks_list[atk_name].level,
            &self.pm.current_player.clone().kind,
        );

        // critical strike
        let is_crit = match self.pm.current_player.process_critical_strike(atk_name) {
            Ok(is_crit) => is_crit,
            Err(e) => {
                tracing::error!(
                    "Error while processing critical strike for player {}: {}",
                    self.pm.current_player.id_name,
                    e
                );
                false
            }
        };
        // process boss target
        self.pm.process_boss_target();

        // ProcessAtk
        let all_effects_param =
            match self
                .pm
                .current_player
                .process_atk(&self.game_state, is_crit, &atk)
            {
                Ok(effects) => effects,
                Err(e) => {
                    tracing::error!(
                        "Error while processing attack {} for player {}: {}",
                        atk_name,
                        self.pm.current_player.id_name,
                        e
                    );
                    vec![]
                }
            };
        // apply effect param on targets
        let launcher_stats = self.pm.current_player.stats.clone();
        let id_name = self.pm.current_player.id_name.clone();
        let kind = self.pm.current_player.kind.clone();
        let mut all_dodging = vec![];
        let launcher_info = LauncherAtkInfo {
            id_name: id_name.clone(),
            kind,
            stats: launcher_stats,
            atk_type: atk.clone(),
        };

        let mut new_gaes: Vec<GameAtkEffect> = Vec::new();
        for processed_effect in &all_effects_param {
            for target_id_name in &all_players {
                let mut gae: Option<GameAtkEffect> = None;
                let mut all_di: Option<Vec<DodgeInfo>> = None;
                if id_name == *target_id_name {
                    (gae, all_di) = self.pm.current_player.is_receiving_atk(
                        processed_effect,
                        &self.game_state,
                        is_crit,
                        &launcher_info,
                    );
                    tracing::trace!(
                        "Effect outcome for self target {}: {:?}",
                        target_id_name,
                        gae
                    );
                } else if let Some(c) = self.pm.get_mut_active_character(target_id_name) {
                    (gae, all_di) = c.is_receiving_atk(
                        processed_effect,
                        &self.game_state,
                        is_crit,
                        &launcher_info,
                    );
                    tracing::trace!("Effect outcome for target {}: {:?}", target_id_name, gae);
                } else {
                    tracing::trace!("Effect outcome for unknown target {}", target_id_name);
                }
                if let Some(mut di) = all_di {
                    all_dodging.append(&mut di);
                };
                if let Some(new_gae) = gae {
                    new_game_atk_effects.push(new_gae.clone());
                    new_gaes.push(new_gae.clone());
                };
            }
        }

        // other function
        // update tx rx
        if is_crit {
            *self.pm.current_player.character_rounds_info.tx_rx
                [AmountType::CriticalStrike as usize]
                .entry(self.game_state.current_turn_nb as u64)
                .or_insert(1) += 1;
        }
        // end of buf

        // new effects to add on the different players
        // RemoveTerminatedEffectsOnPlayer which last only that turn

        // check who died
        self.pm.process_died_players().unwrap_or_else(|e| {
            tracing::error!("Error while processing died players: {}", e);
        });
        // TODO if boss died -> loot

        // update active character for cost atk and buf received.
        self.pm
            .modify_active_character(&self.pm.current_player.id_name.clone());

        // process stats
        self.game_state.process_game_stats(
            &new_gaes,
            &self.pm.current_player.id_name.clone(),
            atk_name,
        );

        // process end of attack
        let mut result_attack = ResultLaunchAttack {
            launcher_id_name: self.pm.current_player.id_name.clone(),
            is_crit,
            new_game_atk_effects: new_game_atk_effects.clone(),
            all_dodging: all_dodging.clone(),
            is_boss_atk: self.pm.current_player.is_boss_atk(),
            logs_end_of_round: Vec::new(),
            logs_atk: self.build_logs_atk(&all_dodging, &new_game_atk_effects, is_crit),
            turn_nb: self.game_state.current_turn_nb,
            round_nb: self.game_state.current_round,
        };

        // eval next step of the game
        result_attack.logs_end_of_round = self.eval_end_of_round(result_attack.logs_atk.clone());

        // update game state with the result of the attack
        self.game_state.last_result_atk = result_attack.clone();

        result_attack
    }

    /// Process end-of-scenario rewards for every hero:
    /// - Add loot items matching the hero's class (equipment checked against the equipment database,
    ///   consumables and currency added directly)
    /// - Add experience gained from all defeated bosses and level up (with stat update) as needed
    /// - Automatically use all consumables in inventory (potions restore HP)
    pub fn process_end_of_scenario(&mut self) {
        // Total exp: sum from all bosses
        let total_exp: u64 = self
            .pm
            .active_bosses
            .iter()
            .map(|boss| build_experience(&boss.rank, boss.level))
            .sum();

        let loots = self.current_scenario.loots.clone();
        let equipment_table_flat: Vec<Equipment> = self
            .pm
            .equipment_table
            .values()
            .flatten()
            .cloned()
            .collect();

        for i in 0..self.pm.active_heroes.len() {
            let hero_class = self.pm.active_heroes[i].class.clone();

            // Add loot according to class
            for loot in &loots {
                let class_matches =
                    loot.class.contains(&hero_class) || loot.class.contains(&Class::Standard);
                if !class_matches {
                    continue;
                }
                match &loot.kind {
                    LootType::Equipment => {
                        if let Some(equipment) = equipment_table_flat
                            .iter()
                            .find(|e| e.unique_name == loot.name)
                            .cloned()
                        {
                            self.pm.active_heroes[i]
                                .inventory
                                .add_equipment(&equipment, false);
                        } else {
                            tracing::warn!(
                                "Equipment '{}' not found in equipment database",
                                loot.name
                            );
                        }
                    }
                    LootType::Consumable => {
                        let hp_amount = match &loot.rank {
                            Rank::Common => 20,
                            Rank::Intermediate => 60,
                            Rank::Advanced => 120,
                        };
                        self.pm.active_heroes[i].inventory.add_potion(
                            &loot.name,
                            hp_amount,
                            loot.rank.clone(),
                        );
                    }
                    LootType::Currency => {
                        self.pm.active_heroes[i].inventory.money += loot.level as u64;
                    }
                    LootType::Material => {}
                }
            }

            // Add experience and level up if needed
            self.pm.active_heroes[i].character_rounds_info.exp += total_exp;
            while self.pm.active_heroes[i].character_rounds_info.exp
                >= self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level
                && self.pm.active_heroes[i].level < ULTIMATE_LEVEL
            {
                self.pm.active_heroes[i].character_rounds_info.exp -= self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level;
                self.pm.active_heroes[i].level += 1;
                self.pm.active_heroes[i].stats.update_stats_to_next_level();
                // Recompute the threshold for the new level
                self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level = build_exp_to_next_level(
                    &self.pm.active_heroes[i].rank,
                    &self.pm.active_heroes[i].class,
                    self.pm.active_heroes[i].level,
                );
            }
        }
    }

    fn process_no_atk_launched(&mut self) -> ResultLaunchAttack {
        // no atk launched
        // update action done in round
        self.pm
            .current_player
            .character_rounds_info
            .actions_done_in_round += 1;
        let logs_atk = vec![LogData {
            message: "No attack launched".to_string(),
            color: DARK_RED.to_string(),
        }];
        // eval next step of the game
        let logs_end_of_round = self.eval_end_of_round(logs_atk.clone());
        ResultLaunchAttack {
            launcher_id_name: self.pm.current_player.id_name.clone(),
            is_boss_atk: self.pm.current_player.is_boss_atk(),
            logs_end_of_round,
            logs_atk,
            turn_nb: self.game_state.current_turn_nb,
            round_nb: self.game_state.current_round,
            ..Default::default()
        }
    }

    /// Evaluate the end of the round by checking if the game is finished,
    ///  if a new round should start or if a new turn should start,
    ///  and return the logs to display for the new round if it is the case
    fn eval_end_of_round(&mut self, logs_atk: Vec<LogData>) -> Vec<LogData> {
        let mut output_logs = vec![];
        let (all_heroes_dead, all_bosses_dead) = self.pm.check_end_of_game();
        if all_heroes_dead {
            self.game_state.status = GameStatus::EndOfGame;
        } else if all_bosses_dead {
            self.game_state.status = GameStatus::EndOfScenario;
            self.process_end_of_scenario();
        } else {
            let (is_new_round, logs) = self.new_round();
            output_logs.extend(logs);
            if is_new_round {
                self.game_state.status = GameStatus::StartRound;
            } else {
                let (is_new_turn, logs) = self.start_new_turn();
                output_logs.extend(logs);
                if is_new_turn {
                    self.game_state.status = GameStatus::StartRound;
                } else {
                    self.game_state.status = GameStatus::EndOfGame;
                }
            }
        }

        self.logs.extend(output_logs.clone());
        self.logs.extend(logs_atk.clone());

        output_logs
    }

    pub fn build_logs_atk(
        &self,
        all_dodging: &Vec<DodgeInfo>,
        all_gae: &Vec<GameAtkEffect>,
        is_crit: bool,
    ) -> Vec<LogData> {
        let mut logs: Vec<LogData> = vec![];
        // dodging and blocking info
        for d in all_dodging {
            tracing::debug!("Dodge info for {}: {:?}", d.name, d);
            if d.is_dodging {
                logs.push(LogData {
                    message: format!("{} is dodging", d.name),
                    color: LIGHT_BLUE.to_string(),
                });
            } else if d.is_blocking {
                logs.push(LogData {
                    message: format!("{} is blocking", d.name),
                    color: LIGHT_GREEN.to_string(),
                });
            }
        }
        // logs for the atk
        if !all_gae.is_empty() {
            logs.push(LogData {
                message: utils::format_string_with_timestamp("Last attack"),
                color: "".to_string(),
            });
            if is_crit {
                logs.push(LogData {
                    message: "Critical strike!".to_string(),
                    color: DARK_RED.to_string(),
                });
            }

            for gae in all_gae {
                // log for the processed effect param
                if !gae.processed_effect_param.log.message.is_empty() {
                    logs.push(gae.processed_effect_param.log.clone());
                }
                // log for the effect outcome
                let mut colortext = LIGHT_GREEN;
                if gae
                    .processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name
                    == HP
                    && gae.effect_outcome.real_amount_tx < 0
                    || gae.effect_outcome.full_amount_tx < 0
                {
                    colortext = DARK_RED;
                }
                if gae.processed_effect_param.input_effect_param.buffer.kind
                    == BufKinds::CooldownTurnsNumber
                {
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: format!(
                            "{} is applying {} on {} for {} turns",
                            gae.effect_outcome.target_id_name,
                            gae.processed_effect_param.input_effect_param.buffer.kind,
                            gae.processed_effect_param
                                .input_effect_param
                                .buffer
                                .stats_name,
                            gae.processed_effect_param.input_effect_param.nb_turns
                        ),
                    });
                } else if gae
                    .processed_effect_param
                    .input_effect_param
                    .buffer
                    .stats_name
                    == HP
                {
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: format!(
                            "{} is applying {} on {} for {} HP",
                            gae.effect_outcome.target_id_name,
                            gae.processed_effect_param.input_effect_param.buffer.kind,
                            gae.processed_effect_param
                                .input_effect_param
                                .buffer
                                .stats_name,
                            gae.effect_outcome.full_amount_tx
                        ),
                    });
                } else {
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: format!(
                            "{} is applying {} on {} for {} {}",
                            gae.effect_outcome.target_id_name,
                            gae.processed_effect_param.input_effect_param.buffer.kind,
                            gae.processed_effect_param
                                .input_effect_param
                                .buffer
                                .stats_name,
                            gae.effect_outcome.full_amount_tx,
                            gae.processed_effect_param
                                .input_effect_param
                                .buffer
                                .stats_name
                        ),
                    });
                }
            }
        }
        logs
    }

    /// Check if it is the turn to a boss to play
    /// HMI function
    pub fn is_round_auto(&self) -> bool {
        if self.game_state.current_round as i64 > 0
            && self.game_state.current_round as i64 - 1 < self.game_state.order_to_play.len() as i64
        {
            let name = self.game_state.order_to_play[self.game_state.current_round - 1].clone();
            if let Some(c) = self.pm.get_active_character(&name) {
                return c.kind == CharacterKind::Boss;
            }
        }

        false
    }

    /// Process the number of bosses that are attacking in a row in the current round, to know if it is the case to add a log for the new round with the info of the boss attack
    /// boss should not be dead to be counted
    /// used by dx-rpg
    pub fn process_nb_bosses_atk_in_a_row(&self) -> i64 {
        let mut count = 0;

        if self.game_state.current_round as i64 > 0
            && self.game_state.current_round as i64 - 1 < self.game_state.order_to_play.len() as i64
        {
            // Start from current_round and go to the end
            for i in self.game_state.current_round - 1..self.game_state.order_to_play.len() {
                let name = &self.game_state.order_to_play[i];

                if let Some(c) = self.pm.get_active_character(name) {
                    if c.kind == CharacterKind::Boss && c.stats.is_dead() != Some(true) {
                        count += 1;
                    } else {
                        break; // Stop counting when a non-Boss is found
                    }
                } else {
                    break; // Stop counting if character doesn't exist
                }
            }
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use crate::character_mod::buffers::BufKinds;
    use crate::character_mod::character::CharacterKind;
    use crate::character_mod::class::Class;
    use crate::character_mod::equipment::Equipment;
    use crate::common::constants::attak_const::COEFF_CRIT_DMG;
    use crate::common::log_data::const_colors::DARK_RED;
    use crate::server::game_manager::LogData;
    use crate::server::game_state::GameStatus;
    use crate::testing::testing_all_characters::{
        self, testing_game_manager, testing_test_ally1_vs_test_boss1,
    };
    use crate::{
        common::constants::{character_const::SPEED_THRESHOLD, stats_const::*},
        testing::testing_atk::*,
    };

    #[test]
    fn unit_process_order_to_play() {
        let mut gm = testing_game_manager();
        let old_speed = gm
            .pm
            .get_mut_active_hero_character("test_#1")
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        gm.process_order_to_play();
        let new_speed = gm
            .pm
            .get_mut_active_hero_character("test_#1")
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        assert_eq!(gm.game_state.order_to_play.len(), 6);
        assert_eq!(gm.game_state.order_to_play[0], "test_#1");
        assert_eq!(gm.game_state.order_to_play[1], "test2_#1");
        assert_eq!(gm.game_state.order_to_play[2], "test_boss1_#1");
        assert_eq!(gm.game_state.order_to_play[3], "test_boss2_#1");
        // supplementary atk
        assert_eq!(gm.game_state.order_to_play[4], "test_#1");
        assert_eq!(gm.game_state.order_to_play[5], "test2_#1");
        assert_eq!(old_speed.current - SPEED_THRESHOLD, new_speed.current);
        assert_eq!(old_speed.max - SPEED_THRESHOLD, new_speed.max);
        assert_eq!(old_speed.max_raw - SPEED_THRESHOLD, new_speed.max_raw);
        assert_eq!(
            old_speed.current_raw - SPEED_THRESHOLD,
            new_speed.current_raw
        );
        // one hero player is dead
        gm.pm.active_heroes[0].stats.all_stats[HP].current = 0;
        gm.process_order_to_play();
        assert_eq!(gm.game_state.order_to_play.len(), 5);
        assert_eq!(gm.game_state.order_to_play[0], "test2_#1");
        assert_eq!(gm.game_state.order_to_play[1], "test_#1");
        assert_eq!(gm.game_state.order_to_play[2], "test_boss1_#1");
        assert_eq!(gm.game_state.order_to_play[3], "test_boss2_#1");
        assert_eq!(gm.game_state.order_to_play[4], "test2_#1");
        // boss is dead
        gm.pm.active_bosses[0].stats.all_stats[HP].current = 0;
        gm.process_order_to_play();
        assert_eq!(gm.game_state.order_to_play.len(), 4);
        assert_eq!(gm.game_state.order_to_play[0], "test2_#1");
        assert_eq!(gm.game_state.order_to_play[1], "test_#1");
        assert_eq!(gm.game_state.order_to_play[2], "test_boss2_#1");
        assert_eq!(gm.game_state.order_to_play[3], "test2_#1");
    }

    #[test]
    fn unit_add_sup_atk_turn() {
        let mut gm = testing_all_characters::testing_game_manager();
        let hero = gm.pm.active_heroes.first_mut().unwrap();
        hero.stats.all_stats.get_mut(SPEED).unwrap().current = 300;
        let boss = gm.pm.active_bosses.first_mut().unwrap();
        boss.stats.all_stats.get_mut(SPEED).unwrap().current = 10;
        let result = gm.pm.process_sup_atk_turn(CharacterKind::Hero);
        // there are 2 allies in the test/offlines to len = 2
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn unit_new_round() {
        let mut gm = testing_all_characters::testing_game_manager();
        let result = gm.start_new_turn();
        assert!(result.0);
        assert_eq!(gm.game_state.current_round, 1);

        // test current player -test- is dead - round for boss is starting
        gm.game_state.current_round = 0;
        gm.pm.active_heroes[0].stats.all_stats[HP].current = 0;
        let result = gm.new_round();
        assert!(result.0);
        assert_eq!(gm.game_state.current_round, 2);
        // test current round > table order to play
        gm.game_state.current_round = 1000;
        let result = gm.new_round();
        assert!(!result.0);
        // character name in orderToplay list is not a player
        gm.game_state.order_to_play.clear();
        gm.game_state.order_to_play.push("unknown".to_owned());
        gm.game_state.current_round = 0;
        let result = gm.new_round();
        assert!(!result.0);
    }

    #[test]
    fn unit_launch_attack_none_atk_hero() {
        let (mut gm, _hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        // test unknown atk
        let ra = gm.launch_attack(None);
        assert_eq!(
            ra.logs_atk,
            vec![LogData {
                message: "No attack launched".to_string(),
                color: DARK_RED.to_string(),
            }]
        );
    }

    #[test]
    fn unit_launch_attack_simple_atk_vigor() {
        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .clone();
        let old_hp_boss = old_boss.stats.all_stats[HP].current;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[VIGOR].current;

        // test normal atk
        // set target
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        let ra = gm.launch_attack(Some("SimpleAtk"));

        assert_eq!(1, ra.new_game_atk_effects.len());
        assert!(ra.all_dodging.is_empty());
        assert!(ra.logs_atk.len() > 0);
        // not dead boss : end of game
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        // vigor dmg: -35(dmg) - 10(phy pow) * 1000/1000+ 5(def phy armor) = -45
        let old_hero_sum_phy_power = gm
            .pm
            .current_player
            .inventory
            .sum_all_equipped_equipment_stat(
                PHYSICAL_POWER,
                &gm.pm
                    .equipment_table
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<Equipment>>(),
            );
        let old_boss_sum_phy_armor = old_boss.inventory.sum_all_equipped_equipment_stat(
            PHYSICAL_ARMOR,
            &gm.pm
                .equipment_table
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Equipment>>(),
        );
        let dmg = (45 + old_hero_sum_phy_power.0) as f64;
        let protection = 1000.0 / (1000.0 + old_boss_sum_phy_armor.0 as f64);
        let atk_amount = dmg * protection;
        assert_eq!(
            std::cmp::max(0, old_hp_boss as i64 - atk_amount as i64) as u64,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // cost: 9 % of vigor 200 = 18
        assert_eq!(
            old_vigor_hero - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[VIGOR]
                .current
        );
    }

    #[test]
    fn unit_launch_attack_simple_atk_vigor_on_dodging_ennemy() {
        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // # case 2 dmg on individual ennemy
        // dodging of boss
        // no critical of current player
        // atk cost is even processed
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 100;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_hp_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[VIGOR].current;
        gm.launch_attack(Some("SimpleAtk"));
        // not dead boss : end of game
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        assert_eq!(
            old_hp_boss,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // 9% of 200 (total vigor)
        assert_eq!(
            old_vigor_hero - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[VIGOR]
                .current
        );
    }

    #[test]
    fn unit_launch_attack_simple_atk_vigor_critical() {
        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // # case 3 dmg on individual ennemy
        // No dodging of boss
        // critical of current player
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 100;
        let old_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .clone();
        let old_hp_boss = old_boss.stats.all_stats[HP].current;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[VIGOR].current;
        gm.launch_attack(Some("SimpleAtk"));
        // 1 dead boss : end of game
        assert!(gm.game_state.status != GameStatus::EndOfGame); // still one boss
        // vigor dmg: -35(dmg) - 10(phy pow) * 1000/1000+ 5(def phy armor) = -45
        // at least coeff critical strike = 2.0 (-45 * 2.0 = -90)
        let old_hero_sum_phy_power = gm
            .pm
            .current_player
            .inventory
            .sum_all_equipped_equipment_stat(
                PHYSICAL_POWER,
                &gm.pm
                    .equipment_table
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<Equipment>>(),
            );
        let old_boss_sum_phy_armor = old_boss.inventory.sum_all_equipped_equipment_stat(
            PHYSICAL_ARMOR,
            &gm.pm
                .equipment_table
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Equipment>>(),
        );
        let dmg = (45 + old_hero_sum_phy_power.0) as f64;
        let protection = 1000.0 / (1000.0 + old_boss_sum_phy_armor.0 as f64);
        let atk_amount = dmg * COEFF_CRIT_DMG * protection;
        assert_eq!(
            std::cmp::max(0, old_hp_boss as i64 - atk_amount as i64) as u64,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // 9% of 200 (total vigor)
        assert_eq!(
            old_vigor_hero - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[VIGOR]
                .current
        );
    }

    #[test]
    fn unit_launch_attack_simple_atk_on_blocking_boss() {
        let (mut gm, hero_launcher_id_name, target_id_name) = testing_test_ally1_vs_test_boss1();

        // # case 4 dmg on individual ennemy
        // No dodging of boss
        // Blocking
        // No critical of current player
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 100;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .class = Class::Berserker;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .clone();
        let old_hp_boss = old_boss.stats.all_stats[HP].current;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[MANA].current;
        let old_hero_sum_phy_power = gm
            .pm
            .current_player
            .inventory
            .sum_all_equipped_equipment_stat(
                PHYSICAL_POWER,
                &gm.pm
                    .equipment_table
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<Equipment>>(),
            );
        let old_boss_sum_phy_armor = old_boss.inventory.sum_all_equipped_equipment_stat(
            PHYSICAL_ARMOR,
            &gm.pm
                .equipment_table
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<Equipment>>(),
        );
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        gm.launch_attack(Some("SimpleAtk"));
        // not dead boss : end of game
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        // vigor dmg: -35(dmg) - 10(phy pow) * 1000/1000+ 5(def phy armor) = -45
        // blocking 10% of the damage is received (10% of 45)
        let dmg = (45 + old_hero_sum_phy_power.0) as f64;
        let protection = 1000.0 / (1000.0 + old_boss_sum_phy_armor.0 as f64);
        let blocking = dmg * protection * 10.0 / 100.0;
        assert_eq!(
            old_hp_boss - blocking as u64,
            gm.pm
                .get_active_boss_character(&target_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // 9% of 200 (total vigor)
        assert_eq!(
            old_vigor_hero - 18,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[VIGOR]
                .current
        );
    }

    #[test]
    fn unit_launch_attack_atk_heal1_zone() {
        // Zone = Tous les heroes
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        // # case 5 up and change on zone ally
        // ally 1 speed > ally 2 speed
        // no critical strike
        let atk = build_atk_heal1_zone();
        gm.pm
            .current_player
            .attacks_list
            .insert(atk.name.clone(), atk.clone());
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_hp_test2 = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mana_launcher = gm.pm.current_player.stats.all_stats[MANA].current;
        gm.launch_attack(Some(&atk.clone().name));
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        // + 30  of max HP:135 = 40
        assert_eq!(
            old_hp_test2 + 40,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // -10% of mana max (see effect param of the atk)
        assert_eq!(
            old_mana_launcher - (0.1 * old_mana_launcher as f64) as u64,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        ); // 10% of 200 (total mana)
    }

    #[test]
    fn unit_launch_attack_case_eclat_despoir() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();
        // no crit
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_hp_test = gm
            .pm
            .get_active_hero_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mag_pow_test = gm
            .pm
            .get_active_hero_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;
        let old_phy_pow_test = gm
            .pm
            .get_active_hero_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        let old_hp_test2 = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mag_pow_test2 = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;
        let old_phy_pow_test2 = gm
            .pm
            .get_active_hero_character("test2_#1")
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        let old_mana_launcher = gm.pm.current_player.stats.all_stats[MANA].current;
        gm.launch_attack(Some("Eclat d'espoir"));
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        // "up-current-stat-by-percentage"
        // + 30 % of max HP:135 = 40.5 + NextAtkHealIsCrit x2 = 80 on test2 and test1
        assert_eq!(
            old_hp_test2 + 40,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        assert_eq!(
            old_hp_test + 40,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // -18%, mana max = 200
        assert_eq!(
            old_mana_launcher - (0.18 * old_mana_launcher as f64) as u64,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        );
        // "Magic power"
        // "ChangeMaxStatByPercentage" 15
        // +15%, mag power max = 20
        assert_eq!(
            old_mag_pow_test2 + (0.15 * old_mag_pow_test2 as f64) as u64,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max
        );
        assert_eq!(
            old_mag_pow_test + (0.15 * old_mag_pow_test as f64) as u64,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max
        );
        // "Physical power"
        // "ChangeMaxStatByPercentage" 15
        // +15%, phy power max = 10
        assert_eq!(
            old_phy_pow_test2 + (0.15 * old_phy_pow_test2 as f64).round() as u64,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[PHYSICAL_POWER]
                .max
        );
        assert_eq!(
            old_phy_pow_test + (0.15 * old_phy_pow_test as f64) as u64,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[PHYSICAL_POWER]
                .max
        );
    }

    #[test]
    fn unit_launch_attack_end_of_effect() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        // turn 1 round 1 (test)
        assert_eq!(gm.game_state.order_to_play.len(), 6);
        assert_eq!(gm.pm.current_player.id_name, hero_launcher_id_name);
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        // apply effect Magic power - up by % for 2 turns (for turn1 and turn2 and is ending on turn 3)
        gm.launch_attack(Some("Eclat d'espoir"));
        // turn 1 round 2 (test2)
        while gm.pm.current_player.id_name != "test2_#1" {
            gm.new_round();
        }
        assert_eq!(gm.pm.current_player.id_name, "test2_#1".to_owned());
        // turn 1 round 3 (boss1)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_boss1_#1".to_owned());
        // turn 1 round 4 (boss2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_boss2_#1".to_owned());
        // turn 1 round 5 (test)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_#1".to_owned());
        // turn 1 round 6 (test2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test2_#1".to_owned());
        // turn 2 round 1
        gm.start_new_turn();
        assert_eq!(gm.pm.current_player.id_name, "test_#1".to_owned());
        // turn 2 round 2 (test2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test2_#1".to_owned());
        // 2 effects received from eclat d espoir (counter turn 1/2, 1 on 2 )
        assert_eq!(
            gm.pm.current_player.character_rounds_info.all_effects.len(),
            2
        );
        // turn 2 round 3 (boss1)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_boss1_#1".to_owned());
        // turn 2 round 4 (boss2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_boss2_#1".to_owned());
        // turn 2 round 5 (test)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test_#1".to_owned());
        // turn 2 round 6 (test2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test2_#1".to_owned());
        // turn 3 round 1 test
        gm.start_new_turn();
        assert_eq!(gm.pm.current_player.id_name, "test_#1".to_owned());
        // turn 3 round 2 (test2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.id_name, "test2_#1".to_owned());
        // effects ended after 2 turns
        assert!(
            gm.pm
                .current_player
                .character_rounds_info
                .all_effects
                .is_empty()
        );
    }

    #[test]
    fn unit_launch_attack_up_par_valeur() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        assert_eq!(gm.pm.current_player.id_name, hero_launcher_id_name);
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_dodge = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .max;
        let result = gm.launch_attack(Some("up-par-valeur"));
        let new_dodge = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[DODGE]
            .max;
        assert_eq!(result.new_game_atk_effects.len(), 1);
        assert_eq!(new_dodge, old_dodge + 20);
    }

    #[test]
    fn unit_launch_attack_changement_par_value_berserk() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_berserk_current = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        let old_berserk_max = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .max;
        let result = gm.launch_attack(Some("ChangeCurrentStatByValue-berseck"));
        let new_berserk = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        assert_eq!(result.new_game_atk_effects.len(), 1); // target himself
        // cost: -5% of berserk max, effect value +20
        assert_eq!(
            new_berserk,
            old_berserk_current - (5 * old_berserk_max / 100) + 20
        );
    }

    #[test]
    fn unit_launch_attack_case_cooldown() {
        let (mut gm, _hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let result = gm.launch_attack(Some("cooldown"));
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        assert_eq!(result.new_game_atk_effects.len(), 1);
        assert_eq!(
            result
                .new_game_atk_effects
                .first()
                .unwrap()
                .processed_effect_param
                .input_effect_param
                .buffer
                .kind,
            BufKinds::CooldownTurnsNumber
        );
    }

    #[test]
    fn unit_integ_dxrpg() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.start_game();
        let old_hp_boss = gm
            .pm
            .get_active_boss_character("Angmar_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        gm.pm
            .get_mut_active_boss_character("Angmar_#1")
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        // thrain
        // game is starting, ennemy is not playing
        assert_eq!(0, gm.process_nb_bosses_atk_in_a_row());
        let ra = gm.launch_attack(Some("SimpleAtk"));
        if !ra.all_dodging.is_empty() && ra.all_dodging[0].is_dodging {
            assert_eq!(
                old_hp_boss,
                gm.pm
                    .get_active_boss_character("Angmar_#1")
                    .unwrap()
                    .stats
                    .all_stats[HP]
                    .current
            );
        } else {
            let mut crit_coeff = 1;
            if ra.is_crit {
                crit_coeff = COEFF_CRIT_DMG as u64;
            }
            assert!(
                old_hp_boss - 31 * crit_coeff
                    >= gm
                        .pm
                        .get_active_boss_character("Angmar_#1")
                        .unwrap()
                        .stats
                        .all_stats[HP]
                        .current
            );
        }
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(2, gm.game_state.current_round);
        // elara
        assert_eq!(0, gm.process_nb_bosses_atk_in_a_row());
        let _ra = gm.launch_attack(Some("SimpleAtk"));
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(3, gm.game_state.current_round);
        let _ra = gm.launch_attack(Some("SimpleAtk"));
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(4, gm.game_state.current_round);
        let _ra = gm.launch_attack(Some("SimpleAtk"));
        assert!(gm.game_state.status != GameStatus::EndOfGame);
        assert_eq!(GameStatus::StartRound, gm.game_state.status);
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(5, gm.game_state.current_round);
        // check if a boss is auto playing
        assert!(gm.is_round_auto());
        assert_eq!(2, gm.process_nb_bosses_atk_in_a_row());
        // None => random atk for boss
        let _ = gm.launch_attack(None); // one or several hero could be dead
        let (all_heroes_dead, all_bosses_dead) = gm.pm.check_end_of_game();
        assert!(!all_heroes_dead);
        assert!(!all_bosses_dead);
        if !all_heroes_dead && !all_bosses_dead {
            assert_eq!(GameStatus::StartRound, gm.game_state.status);
            assert_eq!(1, gm.game_state.current_turn_nb);
            assert_eq!(6, gm.game_state.current_round);
            assert_eq!(1, gm.process_nb_bosses_atk_in_a_row());
            // None => random atk for boss
            let _ = gm.launch_attack(None); // one or several hero could be dead
            let (all_heroes_dead, all_bosses_dead) = gm.pm.check_end_of_game();
            if !all_heroes_dead && !all_bosses_dead {
                assert_eq!(GameStatus::StartRound, gm.game_state.status);
                assert_eq!(2, gm.game_state.current_turn_nb);
                assert_eq!(1, gm.game_state.current_round);
                assert_eq!(0, gm.process_nb_bosses_atk_in_a_row());
            }
        }

        // ensure there is no dead lock -> game can be ended
        while gm.game_state.status == GameStatus::StartRound {
            let _ra = gm.launch_attack(Some("SimpleAtk"));
        }
        assert_eq!(GameStatus::EndOfGame, gm.game_state.status);
    }

    #[test]
    fn unit_launch_attack_boss_pattern_queue() {
        let mut gm = testing_all_characters::testing_game_manager();

        // Set pattern [0, 2] for test_boss1_#1:
        // index 0 = first attack in boss's attacks_list
        // index 2 = third attack in boss's attacks_list
        gm.current_scenario
            .boss_patterns
            .insert("test_boss1_#1".to_string(), vec![0, 2]);

        // start game and navigate to test_boss1_#1's round
        gm.start_game();
        while gm.pm.current_player.id_name != "test_boss1_#1" {
            let (ok, _) = gm.new_round();
            if !ok {
                gm.start_new_turn();
            }
        }

        // queue must be empty before first use
        assert!(
            gm.pm
                .current_player
                .character_rounds_info
                .atk_pattern_queue
                .is_empty(),
            "queue should be empty before first pattern launch"
        );

        // first launch: fills queue with [0, 2], pops 0, boss attacks using atk at index 0
        let ra1 = gm.launch_attack(None);
        assert_ne!(
            ra1.launcher_id_name, "",
            "expected a valid attack to be launched"
        );
        // queue now has [2] stored back in active_bosses
        let queue_after_first: Vec<u64> = gm
            .pm
            .get_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .atk_pattern_queue
            .iter()
            .copied()
            .collect();
        assert_eq!(
            queue_after_first,
            vec![2u64],
            "queue should hold [2] after first launch"
        );

        // navigate back to test_boss1_#1's round
        while gm.pm.current_player.id_name != "test_boss1_#1" {
            let (ok, _) = gm.new_round();
            if !ok {
                gm.start_new_turn();
            }
        }

        // second launch: pops index 2, queue becomes empty
        let ra2 = gm.launch_attack(None);
        assert_ne!(ra2.launcher_id_name, "");
        let queue_after_second: Vec<u64> = gm
            .pm
            .get_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .atk_pattern_queue
            .iter()
            .copied()
            .collect();
        assert!(
            queue_after_second.is_empty(),
            "queue should be empty after second launch"
        );

        // navigate back to test_boss1_#1's round
        while gm.pm.current_player.id_name != "test_boss1_#1" {
            let (ok, _) = gm.new_round();
            if !ok {
                gm.start_new_turn();
            }
        }

        // third launch: queue empty → refills [0, 2], pops 0 again (cycling)
        let ra3 = gm.launch_attack(None);
        assert_ne!(ra3.launcher_id_name, "");
        let queue_after_third: Vec<u64> = gm
            .pm
            .get_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .atk_pattern_queue
            .iter()
            .copied()
            .collect();
        assert_eq!(
            queue_after_third,
            vec![2u64],
            "queue should hold [2] again after cycling"
        );
    }

    #[test]
    fn unit_load_next_scenario() {
        use crate::server::scenario::ScenarioState;

        let mut gm = testing_all_characters::dxrpg_game_manager();

        // dxrpg loads stage_1 and stage_2; states start as NotStarted
        let stage1_name = "Stage 1".to_owned();
        let stage2_name = "Stage 2".to_owned();
        assert_eq!(gm.states_scenarios[&stage1_name], ScenarioState::NotStarted);
        assert_eq!(gm.states_scenarios[&stage2_name], ScenarioState::NotStarted);

        // set stage 1 as current (simulates game start on stage 1)
        let stage1 = gm
            .all_scenarios
            .iter()
            .find(|s| s.name == stage1_name)
            .cloned()
            .unwrap();
        gm.current_scenario = stage1;
        gm.states_scenarios
            .insert(stage1_name.clone(), ScenarioState::InProgress);

        // damage heroes and drain their energy to verify restoration on next scenario
        for hero in gm.pm.active_heroes.iter_mut() {
            hero.stats.get_mut_value(HP).current = 1;
            hero.stats.get_mut_value(MANA).current = 0;
            hero.stats.get_mut_value(VIGOR).current = 0;
            hero.stats.get_mut_value(BERSERK).current = 0;
        }

        // load stage 2
        let result = gm.load_next_scenario();
        assert!(result.is_ok(), "loading stage 2 should succeed");

        // stage 1 must be Completed
        assert_eq!(
            gm.states_scenarios[&stage1_name],
            ScenarioState::Completed,
            "stage 1 should be Completed after loading stage 2"
        );
        // stage 2 must be InProgress
        assert_eq!(
            gm.states_scenarios[&stage2_name],
            ScenarioState::InProgress,
            "stage 2 should be InProgress after being loaded"
        );
        // current_scenario must be stage 2
        assert_eq!(gm.current_scenario.name, stage2_name);

        // active_bosses count must equal the stage 2 boss patterns
        assert_eq!(
            gm.pm.active_bosses.len(),
            gm.current_scenario.boss_patterns.len(),
            "active_bosses should match stage 2 boss patterns count"
        );

        // heroes must have HP, energy and no effects restored to max
        for hero in gm.pm.active_heroes.iter() {
            assert_eq!(
                hero.stats.all_stats[HP].current, hero.stats.all_stats[HP].max,
                "hero {} HP should be restored to max",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[MANA].current, hero.stats.all_stats[MANA].max,
                "hero {} Mana should be restored to max",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[VIGOR].current, hero.stats.all_stats[VIGOR].max,
                "hero {} Vigor should be restored to max",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[BERSERK].current, 0,
                "hero {} Berserk should NOT be restored on scenario load",
                hero.db_full_name
            );
            assert!(
                hero.character_rounds_info.all_effects.is_empty(),
                "hero {} should have no active effects after scenario transition",
                hero.db_full_name
            );
        }

        // all_scenarios_completed returns false (stage 2 still in progress)
        assert!(!gm.all_scenarios_completed());
    }

    #[test]
    fn unit_set_active_bosses() {
        use crate::testing::testing_all_characters::dxrpg_dm;

        let dm = dxrpg_dm();
        let mut gm = testing_all_characters::dxrpg_game_manager();

        // set stage 1 as current scenario so boss_patterns are in scope
        let stage1 = gm
            .all_scenarios
            .iter()
            .find(|s| s.level == 1)
            .cloned()
            .unwrap();
        gm.current_scenario = stage1;

        // no bosses yet
        gm.pm.active_bosses.clear();
        assert_eq!(gm.pm.active_bosses.len(), 0);

        gm.set_active_bosses(&dm.all_bosses);

        // the number of active bosses must match the number of boss_patterns entries
        // that have a matching entry in dm.all_bosses
        let expected = gm
            .current_scenario
            .boss_patterns
            .keys()
            .filter(|name| dm.all_bosses.iter().any(|b| &b.db_full_name == *name))
            .count();
        assert_eq!(
            gm.pm.active_bosses.len(),
            expected,
            "active_bosses count should match boss_patterns with a known boss"
        );

        // each active boss must have the correct id_name suffix format
        for boss in &gm.pm.active_bosses {
            assert!(
                boss.id_name.contains("_#"),
                "id_name '{}' should contain '_#'",
                boss.id_name
            );
        }
    }

    // -------------------------------------------------------------------------
    // process_end_of_scenario tests
    // -------------------------------------------------------------------------

    #[test]
    fn unit_end_of_scenario_equipment_loot_matching_class() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Both test heroes are Standard class.
        // Create a scenario with one equipment loot targeting Standard heroes.
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "starting right weapon".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                class: vec![Class::Standard],
            }],
        };

        gm.process_end_of_scenario();

        // Both heroes must now have the equipment in their inventory
        for hero in &gm.pm.active_heroes {
            let has_it = hero
                .inventory
                .equipments
                .values()
                .flatten()
                .any(|e| e.unique_name == "starting right weapon");
            assert!(
                has_it,
                "hero '{}' should have received the equipment",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_equipment_loot_no_class_match() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Both test heroes are Standard.
        // Equipment loot only for Warrior → heroes must NOT receive an extra copy.
        let belts_before: Vec<usize> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| {
                h.inventory
                    .equipments
                    .values()
                    .flatten()
                    .filter(|e| e.unique_name == "starting belt")
                    .count()
            })
            .collect();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "starting belt".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                class: vec![Class::Warrior],
            }],
        };

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            let belts_after = hero
                .inventory
                .equipments
                .values()
                .flatten()
                .filter(|e| e.unique_name == "starting belt")
                .count();
            assert_eq!(
                belts_after, belts_before[idx],
                "hero '{}' belt count should not change (class mismatch)",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_equipment_loot_unknown_equipment() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Record initial equipment count per hero
        let equip_before: Vec<usize> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.inventory.equipments.values().map(|v| v.len()).sum())
            .collect();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "non_existent_equipment".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                class: vec![Class::Standard],
            }],
        };

        // Must not panic; unknown equipment is just warned about and skipped
        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            let total_equip: usize = hero.inventory.equipments.values().map(|v| v.len()).sum();
            assert_eq!(
                total_equip, equip_before[idx],
                "hero '{}' equipment count should not change for unknown loot name",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_consumable_loot() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "Common potion".to_string(),
                kind: LootType::Consumable,
                rank: Rank::Common, // heals 20 HP
                level: 1,
                class: vec![Class::Standard],
            }],
        };

        gm.process_end_of_scenario();

        for hero in &gm.pm.active_heroes {
            let has_potion = hero
                .inventory
                .consumables
                .iter()
                .any(|c| c.name == "Common potion");
            assert!(
                has_potion,
                "hero '{}' should have received the consumable",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_currency_loot() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "gold".to_string(),
                kind: LootType::Currency,
                rank: Rank::Common,
                level: 100,
                class: vec![Class::Standard],
            }],
        };

        // Test heroes already have money: 100 in their JSON
        let money_before: Vec<u64> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.inventory.money)
            .collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.inventory.money,
                money_before[idx] + 100,
                "hero '{}' should have received 100 gold",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_exp_and_level_up() {
        // Test setup: 2 bosses, each rank Common level 1 → 100 exp each → 200 total
        //
        // "test" hero:  exp=50, exp_to_next_level(Common, Standard, 1)=100
        //   50 + 200 = 250 → level-up to 2 (exp=150), new threshold=200 → 150 < 200 → stop at level 2
        //
        // "test2" hero: exp=0,  exp_to_next_level(Common, Standard, 1)=100
        //   0 + 200 = 200 → level-up to 2 (exp=100), new threshold=200 → 100 < 200 → stop at level 2
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            loots: vec![],
            level: 1,
        };

        let old_hp_max: Vec<u64> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.stats.all_stats[HP].max)
            .collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.level, 2,
                "hero '{}' should be level 2 after 200 exp (dynamic threshold grows to 200 at level 2)",
                hero.id_name
            );
            // exp_to_next_level must now reflect the new level
            assert_eq!(
                hero.character_rounds_info.exp_to_next_level,
                200, // build_exp_to_next_level(Common, Standard, 2) = 200
                "hero '{}' exp_to_next_level should be 200 at level 2",
                hero.id_name
            );
            // Stats must have been updated upward on level-up
            assert!(
                hero.stats.all_stats[HP].max > old_hp_max[idx],
                "hero '{}' HP max should have increased after leveling up",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_no_level_up_when_exp_insufficient() {
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Remove all bosses so total_exp = 0
        gm.pm.active_bosses.clear();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            loots: vec![],
            level: 1,
        };

        let levels_before: Vec<u64> = gm.pm.active_heroes.iter().map(|h| h.level).collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.level, levels_before[idx],
                "hero '{}' should not have leveled up with 0 exp",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_triggered_via_game_flow() {
        // Verify that eval_end_of_round sets EndOfScenario and processes rewards
        // when all bosses are killed in a single hit.
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let (mut gm, _, _) = testing_test_ally1_vs_test_boss1();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "gold".to_string(),
                kind: LootType::Currency,
                rank: Rank::Common,
                level: 50,
                class: vec![Class::Standard],
            }],
        };

        // Kill all bosses
        for boss in gm.pm.active_bosses.iter_mut() {
            boss.stats.all_stats.get_mut(HP).unwrap().current = 0;
        }

        // Set target and launch — eval_end_of_round sees all bosses dead
        gm.pm
            .get_mut_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        gm.launch_attack(None);

        assert_eq!(
            gm.game_state.status,
            GameStatus::EndOfScenario,
            "status should be EndOfScenario"
        );
        // Rewards must have been processed: each Standard hero got 50 gold on top of their starting 100
        for hero in &gm.pm.active_heroes {
            assert!(
                hero.inventory.money >= 50,
                "hero '{}' should have received 50 gold after end-of-scenario",
                hero.id_name
            );
        }
    }
}
