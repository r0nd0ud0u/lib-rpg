use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    attack_type::{AttackType, LauncherAtkInfo},
    character::{AmountType, CharacterType},
    common::{paths_const::*, stats_const::*},
    effect::EffectOutcome,
    game_state::{GameState, GameStatus},
    players_manager::{DodgeInfo, PlayerManager},
    utils,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultLaunchAttack {
    pub launcher_name: String,
    pub outcomes: Vec<EffectOutcome>,
    pub is_crit: bool,
    pub all_dodging: Vec<DodgeInfo>,
    pub is_boss_atk: bool,
    pub logs_new_round: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePaths {
    /// Root path for the game, where all the different files will be stored
    pub root: PathBuf,
    /// Path where the characters of the game are stored
    pub characters: PathBuf,
    /// Path where the equipments of the game are stored
    pub equipments: PathBuf,
    /// Path where the loot of the game are stored
    pub loot: PathBuf,
    /// Path where the ongoing effects of the game are stored
    pub ongoing_effects: PathBuf,
    /// Path where the game state of the game is stored
    pub game_state: PathBuf,
    /// Path where the stats in game of the game are stored
    pub stats_in_game: PathBuf,
    /// Path where the different games are stored
    pub games_dir: PathBuf,
    /// Path where the current game is stored
    pub current_game_dir: PathBuf,
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
}

impl GameManager {
    /// Create a new game manager with the given path for the offline files and the default active characters
    pub fn try_new<P: AsRef<Path>>(
        path: P,
        is_default_active_characters: bool,
    ) -> Result<GameManager> {
        let mut new_path = path.as_ref();
        if new_path.as_os_str().is_empty() {
            new_path = &OFFLINE_ROOT;
        }
        let pm = PlayerManager::try_new(new_path, is_default_active_characters)?;
        Ok(GameManager {
            game_state: GameState::new(),
            pm,
            game_paths: GamePaths {
                root: new_path.to_path_buf(),
                games_dir: GAMES_DIR.to_path_buf(),
                ..Default::default()
            },
        })
    }

    /// Init the game state and build the different paths for the game
    pub fn init_new_game(&mut self) {
        // Init the game state
        self.game_state.init();
        // Build the different paths for the game
        self.build_game_paths();
    }

    /// Start the game by starting a new turn
    pub fn start_game(&mut self) {
        // Start a new turn
        let _ = self.start_new_turn();
    }

    pub fn load_game<P: AsRef<Path>>(&mut self, game_path_dir: P) -> Result<()> {
        self.build_game_paths();
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
    pub fn start_new_turn(&mut self) -> (bool, Vec<String>) {
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
            if !hero.is_dead().unwrap_or(false) {
                self.game_state.order_to_play.push(hero.name.clone());
            } else {
                dead_heroes.push(hero.name.clone());
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
            if !boss.is_dead().unwrap_or(false) {
                self.game_state.order_to_play.push(boss.name.clone());
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
            .all(|c| c.is_dead() == Some(true));
        let all_bosses_dead = self
            .pm
            .active_bosses
            .iter()
            .all(|c| c.is_dead() == Some(true));
        all_bosses_dead || all_heroes_dead
    }

    pub fn new_round(&mut self) -> (bool, Vec<String>) {
        self.game_state.new_round();
        // Still round to play
        if self.game_state.current_round > self.game_state.order_to_play.len() {
            return (false, Vec::new());
        }
        let Ok(logs) = self.pm.update_current_player(
            &self.game_state,
            &self.game_state.order_to_play[self.game_state.current_round - 1],
        ) else {
            return (false, Vec::new());
        };

        if self.pm.current_player.is_dead() == Some(true) {
            self.new_round();
        }

        self.pm.reset_targeted_character();
        // Those 2 TODO are logs to give info
        // TODO case BOSS: random atk to choose
        // TODO who has the most aggro ?

        // TODO update game status
        // TODO channels for logss

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
                if let Some(auto_atk_name) =
                    AttackType::get_one_random_atk_name(&self.pm.current_player.attacks_list)
                {
                    tracing::info!(
                        "Auto attack for boss {}: {}",
                        self.pm.current_player.name,
                        auto_atk_name
                    );
                    return self.launch_attack(Some(&auto_atk_name));
                }
            }
            // update action done in round
            self.pm.current_player.actions_done_in_round += 1;
            tracing::error!(
                "Error: no attack name provided for player {}",
                self.pm.current_player.name
            );
            tracing::error!("launch_attack: is_round_auto: {}", self.is_round_auto());
            return ResultLaunchAttack {
                logs_new_round: vec![format!(
                    "Error: no attack name provided for player {}",
                    self.pm.current_player.name
                )],
                ..Default::default()
            };
        };
        // output
        let mut output: Vec<EffectOutcome> = vec![];
        // update action done in round
        self.pm.current_player.actions_done_in_round += 1;
        // get all players
        let all_players = self.pm.get_all_active_names();
        // get atk
        let atk_list = self.pm.current_player.attacks_list.clone();
        let atk = match atk_list.get(atk_name) {
            Some(atk) => atk.clone(),
            None => {
                return ResultLaunchAttack {
                    logs_new_round: vec![format!(
                        "Error: attack {} not found for player {}",
                        atk_name, self.pm.current_player.name
                    )],
                    ..Default::default()
                };
            } // unknown atk
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
        let is_crit = self.pm.current_player.process_critical_strike(atk_name);
        // process boss target
        self.pm.process_boss_target();

        // ProcessAtk
        let all_effects_param = self
            .pm
            .current_player
            .process_atk(&self.game_state, is_crit, &atk);
        let launcher_stats = self.pm.current_player.stats.clone();
        let name = self.pm.current_player.name.clone();
        let kind = self.pm.current_player.kind.clone();
        let mut all_dodging = vec![];
        let launcher_info = LauncherAtkInfo {
            name: name.clone(),
            kind,
            stats: launcher_stats,
            atk_type: atk.clone(),
        };
        for ep in &all_effects_param {
            for target in &all_players {
                let mut o: Option<EffectOutcome> = None;
                let mut all_di: Option<Vec<DodgeInfo>> = None;
                if name == *target {
                    (o, all_di) = self.pm.current_player.is_receiving_atk(
                        ep,
                        self.game_state.current_turn_nb,
                        is_crit,
                        &launcher_info,
                    );
                } else if let Some(c) = self.pm.get_mut_active_character(target) {
                    (o, all_di) = c.is_receiving_atk(
                        ep,
                        self.game_state.current_turn_nb,
                        is_crit,
                        &launcher_info,
                    );
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
            *self.pm.current_player.tx_rx[AmountType::CriticalStrike as usize]
                .entry(self.game_state.current_turn_nb as u64)
                .or_insert(1) += 1;
        }
        // end of buf

        // new effects to add on the different players
        // RemoveTerminatedEffectsOnPlayer which last only that turn

        // check who died
        self.pm.process_died_players();
        // if boss -> loot
        // handle end of game if all bosses are dead

        // update active character for cost atk and buf received.
        self.pm
            .modify_active_character(&self.pm.current_player.name.clone());

        // process end of attack
        let mut result_attack = ResultLaunchAttack {
            launcher_name: self.pm.current_player.name.clone(),
            is_crit,
            outcomes: output,
            all_dodging,
            is_boss_atk: self.is_boss_atk(),
            logs_new_round: vec![],
        };
        if self.check_end_of_game() {
            self.game_state.status = GameStatus::EndOfGame;
        } else {
            let (is_new_round, logs) = self.new_round();
            if is_new_round {
                self.game_state.status = GameStatus::StartRound;
                result_attack.logs_new_round = logs;
            } else {
                let (is_new_turn, logs) = self.start_new_turn();
                if is_new_turn {
                    result_attack.logs_new_round = logs;
                    self.game_state.status = GameStatus::StartRound;
                } else {
                    self.game_state.status = GameStatus::EndOfGame;
                }
            }
        }

        self.game_state.last_result_atk = result_attack.clone();
        // add basic log
        result_attack.logs_new_round.push(format!(
            "{} launched attack {}{}",
            self.pm.current_player.name,
            atk.name,
            if is_crit { " (critical strike)" } else { "" }
        ));
        result_attack
    }

    pub fn save_game_manager(&self) -> Result<()> {
        // write_to_json
        utils::write_to_json(
            &self,
            self.game_paths.current_game_dir.join("game_manager.json"),
        )?;
        Ok(())
    }

    pub fn create_game_dirs(&self) -> Result<()> {
        if let Err(e) = fs::create_dir_all(&self.game_paths.root) {
            eprintln!("Failed to create directory: {}", e);
        }
        if let Err(e) = fs::create_dir_all(&self.game_paths.characters) {
            eprintln!("Failed to create directory: {}", e);
        }
        if let Err(e) = fs::create_dir_all(&self.game_paths.game_state) {
            eprintln!("Failed to create directory: {}", e);
        }
        if let Err(e) = fs::create_dir_all(&self.game_paths.loot) {
            eprintln!("Failed to create directory: {}", e);
        }
        Ok(())
    }

    pub fn build_game_paths(&mut self) {
        let cur_game_path = self
            .game_paths
            .games_dir
            .join(self.game_state.game_name.clone());
        self.game_paths.current_game_dir = cur_game_path.clone();
        self.game_paths.characters = cur_game_path.join(OFFLINE_CHARACTERS.to_path_buf());
        self.game_paths.equipments = cur_game_path.join(OFFLINE_EQUIPMENT.to_path_buf());
        self.game_paths.game_state = cur_game_path.join(OFFLINE_GAMESTATE.to_path_buf());
        self.game_paths.loot = cur_game_path.join(OFFLINE_LOOT_EQUIPMENT.to_path_buf());
        self.game_paths.ongoing_effects = cur_game_path.join(OFFLINE_EFFECTS.to_path_buf());
        self.game_paths.stats_in_game = cur_game_path.join(GAME_STATE_STATS_IN_GAME.to_path_buf());
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

    pub fn process_nb_bosses_atk_in_a_row(&self) -> i64 {
        let mut count = 0;

        if self.game_state.current_round as i64 > 0
            && self.game_state.current_round as i64 - 1 < self.game_state.order_to_play.len() as i64
        {
            // Start from current_round and go to the end
            for i in self.game_state.current_round - 1..self.game_state.order_to_play.len() {
                let name = &self.game_state.order_to_play[i];

                if let Some(c) = self.pm.get_active_character(name) {
                    if c.kind == CharacterType::Boss {
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
    use crate::common::paths_const;
    use crate::game_manager::ResultLaunchAttack;
    use crate::game_state::GameStatus;
    use crate::players_manager::PlayerManager;
    use crate::utils;
    use crate::{
        common::{character_const::SPEED_THRESHOLD, stats_const::*},
        game_manager::GameManager,
        testing_atk::*,
    };

    #[test]
    fn unit_try_new() {
        assert!(GameManager::try_new("unknown", true).is_err());

        let gm = GameManager::try_new("./tests/offlines", true).unwrap();
        assert_eq!(gm.pm.all_heroes.len(), 2);
        assert_eq!(gm.pm.active_heroes.len(), 2);
        assert_eq!(gm.pm.all_bosses.len(), 2);
        assert_eq!(gm.pm.active_bosses.len(), 2);

        // offline_root by default but no file
        let gm = GameManager::try_new("", false).unwrap();
        assert!(gm.pm.active_heroes.is_empty());
        assert_eq!(gm.pm.active_bosses.len(), 2);
        assert_eq!(gm.pm.all_heroes.len(), 4);
        assert_eq!(gm.pm.all_bosses.len(), 2);

        // offline_root by default with unknown file
        assert!(GameManager::try_new("unknown", true).is_err());

        // offline_root by default
        let gm = GameManager::try_new("", true).unwrap();
        assert_eq!(gm.pm.all_heroes.len(), 4);
        assert_eq!(gm.pm.active_heroes.len(), 4);
        assert_eq!(gm.pm.all_bosses.len(), 2);
        assert_eq!(gm.pm.active_bosses.len(), 2);

        // offline_root by default but no file
        let gm = GameManager::try_new("", false).unwrap();
        assert!(gm.pm.active_heroes.is_empty());
        assert_eq!(gm.pm.active_bosses.len(), 2);
        assert_eq!(gm.pm.all_heroes.len(), 4);
        assert_eq!(gm.pm.all_bosses.len(), 2);
    }

    #[test]
    fn unit_process_order_to_play() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        let old_speed = gm
            .pm
            .get_mut_active_hero_character("test")
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        gm.process_order_to_play();
        let new_speed = gm
            .pm
            .get_mut_active_hero_character("test")
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        assert_eq!(gm.game_state.order_to_play.len(), 6);
        assert_eq!(gm.game_state.order_to_play[0], "test");
        assert_eq!(gm.game_state.order_to_play[1], "test2");
        assert_eq!(gm.game_state.order_to_play[2], "Boss1");
        assert_eq!(gm.game_state.order_to_play[3], "Boss2");
        // supplementary atk
        assert_eq!(gm.game_state.order_to_play[4], "test");
        assert_eq!(gm.game_state.order_to_play[5], "test2");
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
        assert_eq!(gm.game_state.order_to_play[0], "test2");
        assert_eq!(gm.game_state.order_to_play[1], "test");
        assert_eq!(gm.game_state.order_to_play[2], "Boss1");
        assert_eq!(gm.game_state.order_to_play[3], "Boss2");
        assert_eq!(gm.game_state.order_to_play[4], "test2");
        // boss is dead
        gm.pm.active_bosses[0].stats.all_stats[HP].current = 0;
        gm.process_order_to_play();
        assert_eq!(gm.game_state.order_to_play.len(), 4);
        assert_eq!(gm.game_state.order_to_play[0], "test2");
        assert_eq!(gm.game_state.order_to_play[1], "test");
        assert_eq!(gm.game_state.order_to_play[2], "Boss2");
        assert_eq!(gm.game_state.order_to_play[3], "test2");
    }

    #[test]
    fn unit_add_sup_atk_turn() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
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
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
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
    fn unit_launch_attack_case1() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
        gm.start_game();

        // # case 1 dmg on individual ennemy
        // No dodging of boss
        // no critical of current player
        // TODO load atk by json
        let atk = build_atk_damage_indiv();
        gm.pm
            .current_player
            .attacks_list
            .insert(atk.name.clone(), atk.clone());
        gm.pm
            .get_mut_active_boss_character("Boss1")
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_hp_boss = gm
            .pm
            .get_active_boss_character("Boss1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mana_hero = gm.pm.current_player.stats.all_stats[MANA].current;
        let old_hero_name = gm.pm.current_player.name.clone();
        gm.pm
            .get_mut_active_boss_character("Boss1")
            .unwrap()
            .is_current_target = true;
        // test unknown atk
        let ra = gm.launch_attack(None);
        assert_eq!(ResultLaunchAttack::default(), ra);
        // test normal atk
        let ra = gm.launch_attack(Some(&atk.clone().name));
        assert_eq!(1, ra.outcomes.len());
        assert_eq!(1, ra.all_dodging.len());
        assert_eq!("Boss1", ra.all_dodging[0].name);
        assert!(!ra.all_dodging[0].is_dodging);
        // not dead boss : end of game
        assert!(!gm.check_end_of_game());
        assert_eq!(
            old_hp_boss - 40,
            gm.pm
                .get_active_boss_character("Boss1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        assert_eq!(
            old_mana_hero - 20,
            gm.pm
                .get_active_hero_character(&old_hero_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        ); // 10% of 200 (total mana)
    }

    #[test]
    fn unit_launch_attack_case2() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
        gm.start_game();

        // # case 2 dmg on individual ennemy
        // dodging of boss
        // no critical of current player
        // TODO load atk by json
        let atk = build_atk_damage_indiv();
        gm.pm
            .current_player
            .attacks_list
            .insert(atk.name.clone(), atk.clone());
        gm.pm
            .get_mut_active_boss_character("Boss1")
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 100;
        gm.pm
            .get_mut_active_boss_character("Boss1")
            .unwrap()
            .is_current_target = true;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_hp_boss = gm
            .pm
            .get_active_boss_character("Boss1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mana_hero = gm.pm.current_player.stats.all_stats[MANA].current;
        let old_hero_name = gm.pm.current_player.name.clone();
        gm.launch_attack(Some(&atk.clone().name));
        // not dead boss : end of game
        assert!(!gm.check_end_of_game());
        assert_eq!(
            old_hp_boss,
            gm.pm
                .get_active_boss_character("Boss1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        assert_eq!(
            old_mana_hero - 20,
            gm.pm
                .get_active_hero_character(&old_hero_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        ); // 10% of 200 (total mana)
    }

    #[test]
    fn unit_launch_attack_case3() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
        gm.start_game();

        // # case 3 dmg on individual ennemy
        // No dodging of boss
        // critical of current player
        // TODO load atk by json
        let atk = build_atk_damage_indiv();
        gm.pm
            .current_player
            .attacks_list
            .insert(atk.name.clone(), atk.clone());
        gm.pm
            .get_mut_active_boss_character("Boss1")
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 0;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 100;
        let old_hp_boss = gm
            .pm
            .get_active_boss_character("Boss1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        gm.pm
            .get_mut_active_boss_character("Boss1")
            .unwrap()
            .is_current_target = true;
        let old_mana_hero = gm.pm.current_player.stats.all_stats[MANA].current;
        let old_hero_name = gm.pm.current_player.name.clone();
        gm.launch_attack(Some(&atk.clone().name));
        // 1 dead boss : end of game
        // assert!(gm.check_end_of_game());
        // at least coeff critical strike = 2.0 (-40 * 2.0 = -80)
        assert!(
            old_hp_boss - 80
                >= gm
                    .pm
                    .get_active_boss_character("Boss1")
                    .unwrap()
                    .stats
                    .all_stats[HP]
                    .current
        );
        assert_eq!(
            old_mana_hero - 20,
            gm.pm
                .get_active_hero_character(&old_hero_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        ); // 10% of 200 (total mana)
    }

    #[test]
    fn unit_launch_attack_case4() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
        gm.start_game();

        // # case 4 dmg on individual ennemy
        // No dodging of boss
        // Blocking
        // No critical of current player
        // TODO load atk by json
        let atk = build_atk_damage_indiv();
        gm.pm
            .current_player
            .attacks_list
            .insert(atk.name.clone(), atk.clone());
        gm.pm
            .get_mut_active_boss_character("Boss1")
            .unwrap()
            .stats
            .all_stats[DODGE]
            .current = 100;
        gm.pm.get_mut_active_boss_character("Boss1").unwrap().class = Class::Tank;
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_hp_boss = gm
            .pm
            .get_active_boss_character("Boss1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mana_hero = gm.pm.current_player.stats.all_stats[MANA].current;
        let old_hero_name = gm.pm.current_player.name.clone();
        gm.pm
            .get_mut_active_boss_character("Boss1")
            .unwrap()
            .is_current_target = true;
        gm.launch_attack(Some(&atk.clone().name));
        // not dead boss : end of game
        assert!(!gm.check_end_of_game());
        // blocking 10% of the damage is received (10% of 40)
        assert_eq!(
            old_hp_boss - 4,
            gm.pm
                .get_active_boss_character("Boss1")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        assert_eq!(
            old_mana_hero - 20,
            gm.pm
                .get_active_hero_character(&old_hero_name)
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        ); // 10% of 200 (total mana)
    }

    #[test]
    fn unit_launch_attack_case5() {
        // Zone = Tous les heroes
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
        gm.start_game();

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
            .get_active_hero_character("test2")
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
                .get_active_hero_character("test2")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // -10%, mana max = 200
        assert_eq!(
            old_mana_launcher - 20,
            gm.pm
                .get_active_hero_character("test")
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        ); // 10% of 200 (total mana)
    }

    #[test]
    fn unit_launch_attack_case_eclat_despoir() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.pm = PlayerManager::testing_pm();
        gm.init_new_game();
        // turn 1 round 1 (test)
        gm.start_game();
        while gm.pm.current_player.name != "test" {
            gm.new_round();
        }
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_hp_test = gm
            .pm
            .get_active_hero_character("test")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mag_pow_test = gm
            .pm
            .get_active_hero_character("test")
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;
        let old_phy_pow_test = gm
            .pm
            .get_active_hero_character("test")
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        let old_hp_test2 = gm
            .pm
            .get_active_hero_character("test2")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        let old_mag_pow_test2 = gm
            .pm
            .get_active_hero_character("test2")
            .unwrap()
            .stats
            .all_stats[MAGICAL_POWER]
            .max;
        let old_phy_pow_test2 = gm
            .pm
            .get_active_hero_character("test2")
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;
        let old_mana_launcher = gm.pm.current_player.stats.all_stats[MANA].current;
        gm.launch_attack(Some("Eclat d'espoir"));
        assert!(!gm.check_end_of_game());
        // "Changement par %"
        // + 30 % of max HP:135 = 40.5
        assert_eq!(
            old_hp_test2 + 40,
            gm.pm
                .get_active_hero_character("test2")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        assert_eq!(
            old_hp_test + 40,
            gm.pm
                .get_active_hero_character("test")
                .unwrap()
                .stats
                .all_stats[HP]
                .current
        );
        // -18%, mana max = 200
        assert_eq!(
            old_mana_launcher - 36,
            gm.pm
                .get_active_hero_character("test")
                .unwrap()
                .stats
                .all_stats[MANA]
                .current
        );
        // "Magic power"
        // "Up par %" 15
        // +15%, mag power max = 20
        assert_eq!(
            old_mag_pow_test2 + 3,
            gm.pm
                .get_active_hero_character("test2")
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max
        );
        assert_eq!(
            old_mag_pow_test + 3,
            gm.pm
                .get_active_hero_character("test")
                .unwrap()
                .stats
                .all_stats[MAGICAL_POWER]
                .max
        );
        // "Physical power"
        // "Up par %" 15
        // +15%, phy power max = 10
        assert_eq!(
            old_phy_pow_test2 + 1,
            gm.pm
                .get_active_hero_character("test2")
                .unwrap()
                .stats
                .all_stats[PHYSICAL_POWER]
                .max
        );
        assert_eq!(
            old_phy_pow_test + 1,
            gm.pm
                .get_active_hero_character("test")
                .unwrap()
                .stats
                .all_stats[PHYSICAL_POWER]
                .max
        );
    }

    #[test]
    fn unit_launch_attack_end_of_effect() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
        gm.create_game_dirs().unwrap();
        gm.start_game();
        // turn 1 round 1 (test)
        assert_eq!(gm.game_state.order_to_play.len(), 6);
        while gm.pm.current_player.name != "test" {
            gm.new_round();
        }
        assert_eq!(gm.pm.current_player.name, "test".to_owned());
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        // apply effect Magic power - up by % for 2 turns (for turn1 and turn2 and is ending on turn 3)
        gm.launch_attack(Some("Eclat d'espoir"));
        // turn 1 round 2 (test2)
        while gm.pm.current_player.name != "test2" {
            gm.new_round();
        }
        assert_eq!(gm.pm.current_player.name, "test2".to_owned());
        // turn 1 round 3 (boss1)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "Boss1".to_owned());
        // turn 1 round 4 (boss2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "Boss2".to_owned());
        // turn 1 round 5 (test)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "test".to_owned());
        // turn 1 round 6 (test2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "test2".to_owned());
        // turn 2 round 1
        gm.start_new_turn();
        assert_eq!(gm.pm.current_player.name, "test".to_owned());
        // turn 2 round 2 (test2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "test2".to_owned());
        // 2 effects received from eclat d espoir (counter turn 1/2, 1 on 2 )
        assert_eq!(gm.pm.current_player.all_effects.len(), 2);
        // turn 2 round 3 (boss1)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "Boss1".to_owned());
        // turn 2 round 4 (boss2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "Boss2".to_owned());
        // turn 2 round 5 (test)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "test".to_owned());
        // turn 2 round 6 (test2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "test2".to_owned());
        // turn 3 round 1 test
        gm.start_new_turn();
        assert_eq!(gm.pm.current_player.name, "test".to_owned());
        // turn 3 round 2 (test2)
        gm.new_round();
        assert_eq!(gm.pm.current_player.name, "test2".to_owned());
        // effects ended after 2 turns
        assert!(gm.pm.current_player.all_effects.is_empty());
    }

    #[test]
    fn unit_launch_attack_up_par_valeur() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
        gm.create_game_dirs().unwrap();
        // turn 1 round 1 (test)
        gm.start_game();
        while gm.pm.current_player.name != "test" {
            gm.new_round();
        }
        assert_eq!(gm.pm.current_player.name, "test");
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_dodge = gm
            .pm
            .get_mut_active_character("test")
            .unwrap()
            .stats
            .all_stats[DODGE]
            .max;
        let result = gm.launch_attack(Some("up-par-valeur"));
        let new_dodge = gm
            .pm
            .get_mut_active_character("test")
            .unwrap()
            .stats
            .all_stats[DODGE]
            .max;
        assert_eq!(result.outcomes.len(), 1);
        assert_eq!(new_dodge, old_dodge + 20);
    }

    #[test]
    fn unit_launch_attack_changement_par_value_berserk() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.init_new_game();
        gm.create_game_dirs().unwrap();
        // turn 1 round 1 (test)
        gm.start_game();
        while gm.pm.current_player.name != "test" {
            gm.new_round();
        }
        assert_eq!(gm.pm.current_player.name, "test".to_owned());
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let old_berserk = gm
            .pm
            .get_mut_active_character("test")
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        let result = gm.launch_attack(Some("changement-par-valeur-berseck"));
        let new_berserk = gm
            .pm
            .get_mut_active_character("test")
            .unwrap()
            .stats
            .all_stats[BERSERK]
            .current;
        assert_eq!(result.outcomes.len(), 1);
        assert_eq!(new_berserk, old_berserk + 20);
    }

    #[test]
    fn unit_launch_attack_case_cooldown() {
        let mut gm = GameManager::try_new("./tests/offlines", true).unwrap();
        gm.pm = PlayerManager::testing_pm();
        gm.init_new_game();
        // turn 1 round 1 (test)
        gm.start_game();
        while gm.pm.current_player.name != "test" {
            gm.new_round();
        }
        gm.pm.current_player.stats.all_stats[CRITICAL_STRIKE].current = 0;
        let result = gm.launch_attack(Some("cooldown"));
        assert!(!gm.check_end_of_game());
        assert_eq!(result.outcomes.len(), 1);
        assert_eq!(
            result
                .outcomes
                .first()
                .unwrap()
                .new_effect_param
                .effect_type,
            EFFECT_NB_COOL_DOWN
        );
    }

    #[test]
    fn unit_integ_dxrpg() {
        let mut gm = GameManager::try_new("offlines", true).unwrap();
        gm.init_new_game();
        gm.create_game_dirs().unwrap();
        gm.start_game();
        let old_hp_boss = gm
            .pm
            .get_active_boss_character("Angmar")
            .unwrap()
            .stats
            .all_stats[HP]
            .current;
        gm.pm
            .get_mut_active_boss_character("Angmar")
            .unwrap()
            .is_current_target = true;
        // thrain
        // game is starting, ennemy is not playing
        assert_eq!(0, gm.process_nb_bosses_atk_in_a_row());
        let ra = gm.launch_attack(Some("SimpleAtk"));
        if !ra.all_dodging.is_empty() && ra.all_dodging[0].is_dodging {
            assert_eq!(
                old_hp_boss,
                gm.pm
                    .get_active_boss_character("Angmar")
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
                        .get_active_boss_character("Angmar")
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

        // check save game
        let path = paths_const::GAMES_DIR.to_path_buf();
        let big_list = utils::list_dirs_in_dir(path);
        let one_save = big_list.unwrap()[0].clone();
        let result = gm.load_game("");
        assert!(result.is_err());
        let _ = gm.load_game(one_save);
        let _ = gm.save_game_manager();
    }
}
