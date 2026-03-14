use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    attack_type::{AttackType, LauncherAtkInfo},
    character::{AmountType, CharacterType},
    common::{effect_const::EFFECT_NB_COOL_DOWN, paths_const::*, stats_const::*},
    effect::EffectOutcome,
    equipment::{Equipment, EquipmentJsonKey},
    game_state::{GameState, GameStatus},
    players_manager::{DodgeInfo, PlayerManager},
    utils,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultLaunchAttack {
    pub launcher_id_name: String,
    pub outcomes: Vec<EffectOutcome>,
    pub is_crit: bool,
    pub all_dodging: Vec<DodgeInfo>,
    pub is_boss_atk: bool,
    pub logs_end_of_round: Vec<LogData>,
    pub logs_atk: Vec<LogData>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogData {
    pub message: String,
    pub color: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePaths {
    /// Root path for the game, where all the different files will be stored
    pub input_data_root: PathBuf,
    /// Path where the characters of the game are stored
    pub input_data_characters: PathBuf,
    /// Path where the equipments of the game are stored
    pub input_data_equipments: PathBuf,
    /// Path where the loot of the game are stored
    pub output_loot: PathBuf,
    /// Path where the ongoing effects of the game are stored
    pub output_ongoing_effects: PathBuf,
    /// Path where the game state of the game is stored
    pub output_game_state: PathBuf,
    /// Path where the stats in game of the game are stored
    pub output_stats_in_game: PathBuf,
    /// Path where the different games are stored
    pub output_games_dir: PathBuf,
    /// Path where the current game is stored
    pub output_current_game_dir: PathBuf,
}

impl GamePaths {
    pub fn new<P: AsRef<Path>>(data_path: P, game_name: &str) -> GamePaths {
        // join GAMES_DIR with game_name to create the current game dir
        let output_dir = GAMES_DIR.to_path_buf().join(game_name);
        GamePaths {
            input_data_root: data_path.as_ref().to_path_buf(),
            output_games_dir: GAMES_DIR.to_path_buf(),
            output_current_game_dir: output_dir.clone(),
            input_data_characters: data_path.as_ref().join(OFFLINE_CHARACTERS.to_path_buf()),
            input_data_equipments: data_path.as_ref().join(OFFLINE_EQUIPMENT.to_path_buf()),
            output_game_state: output_dir.join(OFFLINE_GAMESTATE.to_path_buf()),
            output_loot: output_dir.join(OFFLINE_LOOT_EQUIPMENT.to_path_buf()),
            output_ongoing_effects: output_dir.join(OFFLINE_EFFECTS.to_path_buf()),
            output_stats_in_game: output_dir.join(GAME_STATE_STATS_IN_GAME.to_path_buf()),
        }
    }
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
}

impl GameManager {
    /// Create a new game manager with the given path for the offline files and the default active characters
    pub fn new<P: AsRef<Path>>(
        path: P,
        equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>,
    ) -> GameManager {
        // if path is empty, use the default one
        let mut new_path = path.as_ref();
        if new_path.as_os_str().is_empty() {
            new_path = &OFFLINE_ROOT;
        }
        // create game state
        let game_state = GameState::new();
        let game_name = game_state.game_name.clone();

        GameManager {
            game_state,
            pm: PlayerManager::new(equipment_table),
            game_paths: GamePaths::new(new_path, &game_name),
            logs: Vec::new(),
        }
    }

    /// Start the game by starting a new turn
    pub fn start_game(&mut self) {
        // Start a new turn
        let _ = self.start_new_turn();
    }

    /// TODO use the one from dxrpg
    pub fn load_game<P: AsRef<Path>>(&mut self, game_path_dir: P) -> Result<()> {
        self.game_state =
            utils::read_from_json(game_path_dir.as_ref().join(OFFLINE_GAMESTATE.to_path_buf()))?;
        self.pm
            .load_active_characters_from_saved_game(&game_path_dir)?;
        Ok(())
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

        // TODO update game status
        // TODO init target view
        // TODO add channel for the logs
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
        let supp_rounds_heroes = self.pm.compute_sup_atk_turn(CharacterType::Hero);
        let supp_rounds_bosses = self.pm.compute_sup_atk_turn(CharacterType::Boss);
        self.game_state.order_to_play.extend(supp_rounds_heroes);
        self.game_state.order_to_play.extend(supp_rounds_bosses);
    }

    pub fn check_end_of_game(&self) -> bool {
        let all_heroes_dead = self
            .pm
            .active_heroes
            .iter()
            .all(|c| c.stats.is_dead() == Some(true));
        let all_bosses_dead = self
            .pm
            .active_bosses
            .iter()
            .all(|c| c.stats.is_dead() == Some(true));
        all_bosses_dead || all_heroes_dead
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
        let Ok(logs) = self.pm.update_current_player(
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
        // Those 2 TODO are logs to give info
        // TODO case BOSS: random atk to choose
        // TODO who has the most aggro ?

        // TODO update game status

        // reinit each round

        (true, logs)
    }

    pub fn is_boss_atk(&self) -> bool {
        self.pm.current_player.kind == CharacterType::Boss
    }

    /// Launch an attack from the current player
    /// If atk_name is None and it is an auto round (boss), a random atk will be chosen
    /// Otherwise, if atk_name is None, no atk will be launched
    pub fn launch_attack(&mut self, atk_name: Option<&str>) -> ResultLaunchAttack {
        // is atk existing?
        let Some(atk_name) = atk_name else {
            if self.is_round_auto() {
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
        let mut output: Vec<EffectOutcome> = vec![];
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
        for processed_effect in &all_effects_param {
            for target_id_name in &all_players {
                let mut o: Option<EffectOutcome> = None;
                let mut all_di: Option<Vec<DodgeInfo>> = None;
                if id_name == *target_id_name {
                    (o, all_di) = self.pm.current_player.is_receiving_atk(
                        processed_effect,
                        self.game_state.current_turn_nb,
                        is_crit,
                        &launcher_info,
                    );
                    tracing::trace!("Effect outcome for self target {}: {:?}", target_id_name, o);
                } else if let Some(c) = self.pm.get_mut_active_character(target_id_name) {
                    (o, all_di) = c.is_receiving_atk(
                        processed_effect,
                        self.game_state.current_turn_nb,
                        is_crit,
                        &launcher_info,
                    );
                    tracing::trace!("Effect outcome for target {}: {:?}", target_id_name, o);
                } else {
                    tracing::trace!("Effect outcome for unknown target {}", target_id_name);
                }
                if let Some(mut di) = all_di {
                    all_dodging.append(&mut di);
                };
                if let Some(eo) = o {
                    output.push(eo);
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

        // process end of attack
        let mut result_attack = ResultLaunchAttack {
            launcher_id_name: self.pm.current_player.id_name.clone(),
            is_crit,
            outcomes: output.clone(),
            all_dodging: all_dodging.clone(),
            is_boss_atk: self.is_boss_atk(),
            logs_end_of_round: Vec::new(),
            logs_atk: self.build_logs_atk(&all_dodging, &output, is_crit),
        };

        // eval next step of the game
        result_attack.logs_end_of_round = self.eval_end_of_round(result_attack.logs_atk.clone());

        // update game state with the result of the attack
        self.game_state.last_result_atk = result_attack.clone();

        result_attack
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
            color: "red".to_string(),
        }];
        // eval next step of the game
        let logs_end_of_round = self.eval_end_of_round(logs_atk.clone());
        ResultLaunchAttack {
            launcher_id_name: self.pm.current_player.id_name.clone(),
            is_boss_atk: self.is_boss_atk(),
            logs_end_of_round,
            logs_atk,
            ..Default::default()
        }
    }

    /// Evaluate the end of the round by checking if the game is finished,
    ///  if a new round should start or if a new turn should start,
    ///  and return the logs to display for the new round if it is the case
    fn eval_end_of_round(&mut self, logs_atk: Vec<LogData>) -> Vec<LogData> {
        let mut output_logs = vec![];
        if self.check_end_of_game() {
            self.game_state.status = GameStatus::EndOfGame;
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
        effects_outcomes: &Vec<EffectOutcome>,
        is_crit: bool,
    ) -> Vec<LogData> {
        let mut logs: Vec<LogData> = vec![];
        // dodging and blocking info
        for d in all_dodging {
            tracing::debug!("Dodge info for {}: {:?}", d.name, d);
            if d.is_dodging {
                logs.push(LogData {
                    message: format!("{} is dodging", d.name),
                    color: "#1a73e8".to_string(),
                });
            } else if d.is_blocking {
                logs.push(LogData {
                    message: format!("{} is blocking", d.name),
                    color: "#10b981".to_string(),
                });
            }
        }
        // logs for the atk
        if !effects_outcomes.is_empty() {
            logs.push(LogData {
                message: utils::format_string_with_timestamp("Last attack"),
                color: "".to_string(),
            });
            if is_crit {
                logs.push(LogData {
                    message: "Critical strike!".to_string(),
                    color: "#9b1c1c".to_string(),
                });
            }

            for eo in effects_outcomes {
                // log for the processed effect param
                if !eo.processed_effect_param.log.message.is_empty() {
                    logs.push(eo.processed_effect_param.log.clone());
                }
                // log for the effect outcome
                let mut colortext = "#10b981";
                if eo.processed_effect_param.input_effect_param.stats_name == HP
                    && eo.real_hp_amount_tx < 0
                    || eo.full_atk_amount_tx < 0
                {
                    colortext = "#9b1c1c";
                }
                if eo.processed_effect_param.input_effect_param.effect_type == EFFECT_NB_COOL_DOWN {
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: format!(
                            "{} is applying {} on {} for {} turns",
                            eo.target_kind,
                            eo.processed_effect_param.input_effect_param.effect_type,
                            eo.processed_effect_param.input_effect_param.stats_name,
                            eo.processed_effect_param.input_effect_param.nb_turns
                        ),
                    });
                } else if eo.processed_effect_param.input_effect_param.stats_name == HP {
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: format!(
                            "{} is applying {} on {} for {} HP",
                            eo.target_kind,
                            eo.processed_effect_param.input_effect_param.effect_type,
                            eo.processed_effect_param.input_effect_param.stats_name,
                            eo.full_atk_amount_tx
                        ),
                    });
                } else {
                    logs.push(LogData {
                        color: colortext.to_string(),
                        message: format!(
                            "{} is applying {} on {} for {} {}",
                            eo.target_kind,
                            eo.processed_effect_param.input_effect_param.effect_type,
                            eo.processed_effect_param.input_effect_param.stats_name,
                            eo.full_atk_amount_tx,
                            eo.processed_effect_param.input_effect_param.stats_name
                        ),
                    });
                }
            }
        }
        logs
    }

    /// TODO use that one in dxrpg
    pub fn save_game_manager(&self) -> Result<()> {
        // write_to_json
        utils::write_to_json(
            &self,
            self.game_paths
                .output_current_game_dir
                .join("game_manager.json"),
        )?;
        Ok(())
    }

    pub fn create_game_dirs(&self) -> Result<()> {
        if let Err(e) = fs::create_dir_all(&self.game_paths.input_data_root) {
            eprintln!("Failed to create directory: {}", e);
        }
        if let Err(e) = fs::create_dir_all(&self.game_paths.input_data_characters) {
            eprintln!("Failed to create directory: {}", e);
        }
        if let Err(e) = fs::create_dir_all(&self.game_paths.output_game_state) {
            eprintln!("Failed to create directory: {}", e);
        }
        if let Err(e) = fs::create_dir_all(&self.game_paths.output_loot) {
            eprintln!("Failed to create directory: {}", e);
        }
        Ok(())
    }

    /// Check if it is the turn to a boss to play
    /// HMI function
    pub fn is_round_auto(&self) -> bool {
        if self.game_state.current_round as i64 > 0
            && self.game_state.current_round as i64 - 1 < self.game_state.order_to_play.len() as i64
        {
            let name = self.game_state.order_to_play[self.game_state.current_round - 1].clone();
            if let Some(c) = self.pm.get_active_character(&name) {
                return c.kind == CharacterType::Boss;
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
                    if c.kind == CharacterType::Boss && c.stats.is_dead() != Some(true) {
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
    use crate::character::Class;
    use crate::common::attak_const::COEFF_CRIT_DMG;
    use crate::common::effect_const::EFFECT_NB_COOL_DOWN;
    use crate::game_manager::LogData;
    use crate::game_state::GameStatus;
    use crate::testing_all_characters::{
        self, testing_game_manager, testing_test_ally1_vs_test_boss1,
    };
    use crate::{
        common::{character_const::SPEED_THRESHOLD, stats_const::*},
        testing_atk::*,
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
        let result = gm
            .pm
            .compute_sup_atk_turn(crate::character::CharacterType::Hero);
        // there are 2 allies in the test/offlines to len = 2
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn unit_new_round() {
        let mut gm = testing_all_characters::testing_game_manager();
        let result = gm.start_new_turn();
        assert!(result.0);
        assert_eq!(gm.game_state.current_round, 1);
        // TODO add second hero player to test the current round and avoid auto atk of boss

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
                color: "red".to_string(),
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
        let old_hp_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[VIGOR].current;

        // test normal atk
        // set target
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        let ra = gm.launch_attack(Some("SimpleAtk"));

        assert_eq!(1, ra.outcomes.len());
        assert!(ra.all_dodging.is_empty());
        assert!(ra.logs_atk.len() > 0);
        // not dead boss : end of game
        assert!(!gm.check_end_of_game());
        // vigor dmg: -35(dmg) - 10(phy pow) * 1000/1000+ 5(def phy armor) = -45
        assert_eq!(
            old_hp_boss - 45,
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
        assert!(!gm.check_end_of_game());
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
        let old_hp_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[VIGOR].current;
        gm.launch_attack(Some("SimpleAtk"));
        // 1 dead boss : end of game
        assert!(!gm.check_end_of_game()); // still one boss
        // vigor dmg: -35(dmg) - 10(phy pow) * 1000/1000+ 5(def phy armor) = -45
        // at least coeff critical strike = 2.0 (-45 * 2.0 = -90)
        assert_eq!(
            old_hp_boss - 90,
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
            .class = Class::Tank;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_hp_boss = gm
            .pm
            .get_active_boss_character(&target_id_name)
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_vigor_hero = gm.pm.current_player.stats.all_stats[MANA].current;
        gm.pm
            .get_mut_active_boss_character(&target_id_name)
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        gm.launch_attack(Some("SimpleAtk"));
        // not dead boss : end of game
        assert!(!gm.check_end_of_game());
        // vigor dmg: -35(dmg) - 10(phy pow) * 1000/1000+ 5(def phy armor) = -45
        // blocking 10% of the damage is received (10% of 45)
        assert_eq!(
            old_hp_boss - 4,
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
        assert!(!gm.check_end_of_game());
        // + 30  of max HP:135 = 40.5
        assert_eq!(
            old_hp_test2 + 40,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // -10%, mana max = 200
        assert_eq!(
            old_mana_launcher - 20,
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
        assert!(!gm.check_end_of_game());
        // "up-current-stat-by-percentage"
        // + 30 % of max HP:135 = 40.5
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
            old_mana_launcher - 36,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        );
        // "Magic power"
        // "up-max-stat-by-percentage" 15
        // +15%, mag power max = 20
        assert_eq!(
            old_mag_pow_test2 + 3,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max
        );
        assert_eq!(
            old_mag_pow_test + 3,
            gm.pm
                .get_active_hero_character(&hero_launcher_id_name)
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max
        );
        // "Physical power"
        // "up-max-stat-by-percentage" 15
        // +15%, phy power max = 10
        assert_eq!(
            old_phy_pow_test2 + 1,
            gm.pm
                .get_active_hero_character("test2_#1")
                .unwrap()
                .stats
                .all_stats[PHYSICAL_POWER]
                .max
        );
        assert_eq!(
            old_phy_pow_test + 1,
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
        assert!(gm.pm.current_player.all_effects.is_empty());
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
        assert_eq!(result.outcomes.len(), 1);
        assert_eq!(new_dodge, old_dodge + 20);
    }

    #[test]
    fn unit_launch_attack_changement_par_value_berserk() {
        let (mut gm, hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_berserk = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        let result = gm.launch_attack(Some("change-current-stat-by-value-berseck"));
        let new_berserk = gm
            .pm
            .get_mut_active_character(&hero_launcher_id_name)
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        assert_eq!(result.outcomes.len(), 1); // target himself
        // cost: -5% of 200 = -10, effect value +20 => +10
        assert_eq!(new_berserk, old_berserk + 10);
    }

    #[test]
    fn unit_launch_attack_case_cooldown() {
        let (mut gm, _hero_launcher_id_name, _target_id_name) = testing_test_ally1_vs_test_boss1();

        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let result = gm.launch_attack(Some("cooldown"));
        assert!(!gm.check_end_of_game());
        assert_eq!(result.outcomes.len(), 1);
        assert_eq!(
            result
                .outcomes
                .first()
                .unwrap()
                .processed_effect_param
                .input_effect_param
                .effect_type,
            EFFECT_NB_COOL_DOWN
        );
    }

    #[test]
    fn unit_integ_dxrpg() {
        let mut gm = testing_all_characters::dxrpg_game_manager();
        gm.create_game_dirs().unwrap();
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
        assert!(!gm.check_end_of_game());
        assert_eq!(GameStatus::StartRound, gm.game_state.status);
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(5, gm.game_state.current_round);
        // check if a boss is auto playing
        assert!(gm.is_round_auto());
        assert_eq!(2, gm.process_nb_bosses_atk_in_a_row());
        // None => random atk for boss
        let _ = gm.launch_attack(None); // one or several hero could be dead
        if !gm.check_end_of_game() {
            assert_eq!(GameStatus::StartRound, gm.game_state.status);
            assert_eq!(1, gm.game_state.current_turn_nb);
            assert_eq!(6, gm.game_state.current_round);
            assert_eq!(1, gm.process_nb_bosses_atk_in_a_row());
            // None => random atk for boss
            let _ = gm.launch_attack(None); // one or several hero could be dead
            if !gm.check_end_of_game() {
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

        // TODO case heroes are killing both bosses
    }
}
