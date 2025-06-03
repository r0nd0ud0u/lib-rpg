use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    character::{AmountType, CharacterType},
    common::{paths_const::*, stats_const::*},
    effect::EffectOutcome,
    game_state::{GameState, GameStatus, ResultAtks},
    players_manager::{DodgeInfo, PlayerManager},
    utils,
};
use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultLaunchAttack {
    pub launcher_name: String,
    pub outcomes: Vec<EffectOutcome>,
    pub is_crit: bool,
    pub all_dodging: Vec<DodgeInfo>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePaths {
    pub root: PathBuf,
    pub characters: PathBuf,
    pub equipments: PathBuf,
    pub loot: PathBuf,
    pub ongoing_effects: PathBuf,
    pub game_state: PathBuf,
    pub stats_in_game: PathBuf,
    pub games_dir: PathBuf,
    pub current_game_dir: PathBuf,
}

/// The entry of the library.
/// That object should be called to access to all the different functionalities.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManager {
    pub game_state: GameState,
    /// Player manager
    pub pm: PlayerManager,
    /// Paths of the current game
    pub game_paths: GamePaths,
}

impl GameManager {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<GameManager> {
        let mut new_path = path.as_ref();
        if new_path.as_os_str().is_empty() {
            new_path = &OFFLINE_ROOT;
        }
        let pm = PlayerManager::try_new(new_path)?;
        Ok(GameManager {
            game_state: GameState::new(),
            pm,
            game_paths: GamePaths {
                root: new_path.to_path_buf(),
                games_dir: new_path.to_path_buf().join(GAMES_DIR.to_path_buf()),
                ..Default::default()
            },
        })
    }
    pub fn start_new_game(&mut self) {
        self.game_state.init();
        self.build_game_paths();
    }

    pub fn load_game<P: AsRef<Path>>(&mut self, game_path_dir: P) -> Result<()> {
        self.build_game_paths();
        self.game_state =
            utils::read_from_json(game_path_dir.as_ref().join(OFFLINE_GAMESTATE.to_path_buf()))?;
        self.pm.load_active_characters(&game_path_dir, true)?;
        Ok(())
    }

    pub fn start_new_turn(&mut self) -> bool {
        // For each turn now
        // Process the order of the players
        self.process_order_to_play();

        self.game_state.start_new_turn();

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
            self.game_state.order_to_play.push(boss.name.clone());
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
        self.pm.active_bosses.is_empty() || all_heroes_dead
    }

    pub fn new_round(&mut self) -> bool {
        self.game_state.new_round();
        // Still round to play
        if self.game_state.current_round > self.game_state.order_to_play.len() {
            return false;
        }
        let old_kind = self.pm.current_player.kind.clone();
        if self
            .pm
            .update_current_player(
                &self.game_state,
                &self.game_state.order_to_play[self.game_state.current_round - 1],
            )
            .is_err()
        {
            return false;
        }
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
        if !self.is_auto_atk() {
            // reset
            self.game_state.last_result_atks = ResultAtks::default();
            // init
            self.game_state.last_result_atks.uuid = Uuid::new_v4().to_string();
        }
        // in case of auto atk in a row -> accumulate
        if self.is_auto_atk() {
            self.game_state.last_result_atks.is_auto_atk = true;
            let _ = self.launch_attack("SimpleAtk");
        } else if self.is_end_of_auto_atk(&old_kind) {
            self.pm.reset_auto_atk_info();
        }

        true
    }

    pub fn is_auto_atk(&self) -> bool {
        self.pm.current_player.kind == CharacterType::Boss
    }

    pub fn is_end_of_auto_atk(&self, old_kind: &CharacterType) -> bool {
        *old_kind == CharacterType::Hero
    }

    /**
     * @brief GameDisplay::LaunchAttak
     * Atk of the launcher is processed first to enable the potential bufs
     * then the effets are processed on the other targets(ennemy and allies)
     */
    pub fn launch_attack(&mut self, atk_name: &str) -> ResultLaunchAttack {
        let mut output: Vec<EffectOutcome> = vec![];
        let all_players = self.pm.get_all_active_names();
        self.pm.current_player.actions_done_in_round += 1;
        if !self.pm.current_player.attacks_list.contains_key(atk_name) {
            // TODO log
            return ResultLaunchAttack::default();
        }

        if !self.pm.current_player.attacks_list.contains_key(atk_name) {
            // TODO log
            return ResultLaunchAttack::default();
        }
        self.pm.current_player.process_atk_cost(atk_name);

        // is dodging ?
        self.pm.process_all_dodging(
            &all_players,
            self.pm.current_player.attacks_list[atk_name].level.into(),
        );

        // critical strike
        let is_crit = self.pm.current_player.process_critical_strike(atk_name);
        // process boss target
        self.pm.process_boss_target();

        // ProcessAtk
        let atk_list = self.pm.current_player.attacks_list.clone();
        let atk = if let Some(atk) = atk_list.get(atk_name) {
            atk
        } else {
            return ResultLaunchAttack::default();
        };
        let all_effects_param = self
            .pm
            .current_player
            .process_atk(&self.game_state, is_crit, atk);
        let launcher_stats = self.pm.current_player.stats.clone();
        let name = self.pm.current_player.name.clone();
        let kind = self.pm.current_player.kind.clone();
        let mut all_dodging = vec![];
        for ep in &all_effects_param {
            for target in &all_players {
                if let Some(c) = self.pm.get_mut_active_character(target) {
                    if c.is_dead() == Some(true) {
                        continue;
                    }
                    // update tmp current stats
                    c.stats.sync_tmp_current_value();
                    // check if the effect is applied on the target
                    if c.is_targeted(ep, &name, &kind) {
                        // TODO check if the effect is not already applied
                        output.push(c.apply_effect_outcome(
                            ep,
                            &launcher_stats,
                            is_crit,
                            self.game_state.current_turn_nb,
                        ));
                        // assess the blocking
                        all_dodging.push(c.dodge_info.clone());
                    }
                    // assess the dodging
                    if c.is_dodging(&ep.target) && c.kind != kind && c.is_current_target {
                        all_dodging.push(c.dodge_info.clone());
                    }
                }
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

        self.pm
            .modify_active_character(&self.pm.current_player.name.clone());

        // process end of attack
        let result_attack = ResultLaunchAttack {
            launcher_name: self.pm.current_player.name.clone(),
            is_crit,
            outcomes: output,
            all_dodging,
        };
        if self.check_end_of_game() {
            self.game_state.status = GameStatus::EndOfGame;
        } else if self.new_round() {
            self.game_state.status = GameStatus::StartRound;
        } else {
            self.start_new_turn();
            self.game_state.status = GameStatus::StartRound;
        }

        self.game_state.last_result_atks.nb_atk_stored += 1;

        self.game_state
            .last_result_atks
            .results
            .push(result_attack.clone());

        result_attack
    }

    pub fn save_game_manager(&self) -> Result<()> {
        // write_to_json
        utils::write_to_json(
            &self,
            self.game_paths
                .current_game_dir
                .join("game_manager.json")
                .to_path_buf(),
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
}

#[cfg(test)]
mod tests {
    use crate::character::Class;
    use crate::common::paths_const::{self, OFFLINE_ROOT};
    use crate::utils;
    use crate::{
        common::{character_const::SPEED_THRESHOLD, stats_const::*},
        game_manager::GameManager,
        testing_atk::*,
    };

    #[test]
    fn unit_try_new() {
        assert!(GameManager::try_new("unknown").is_err());

        let gm = GameManager::try_new("./tests/offlines").unwrap();
        assert_eq!(gm.pm.all_heroes.len(), 1);
    }

    #[test]
    fn unit_process_order_to_play() {
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        let old_speed = gm
            .pm
            .active_heroes
            .first()
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        gm.process_order_to_play();
        let new_speed = gm
            .pm
            .active_heroes
            .first()
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        assert_eq!(gm.game_state.order_to_play.len(), 3);
        assert_eq!(gm.game_state.order_to_play[0], "test");
        assert_eq!(gm.game_state.order_to_play[1], "Boss1");
        // supplementary atk
        assert_eq!(gm.game_state.order_to_play[2], "test");

        assert_eq!(old_speed.current - SPEED_THRESHOLD, new_speed.current);
        assert_eq!(old_speed.max - SPEED_THRESHOLD, new_speed.max);
        assert_eq!(old_speed.max_raw - SPEED_THRESHOLD, new_speed.max_raw);
        assert_eq!(
            old_speed.current_raw - SPEED_THRESHOLD,
            new_speed.current_raw
        );
    }

    #[test]
    fn unit_add_sup_atk_turn() {
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        let hero = gm.pm.active_heroes.first_mut().unwrap();
        hero.stats.all_stats.get_mut(SPEED).unwrap().current = 300;
        let boss = gm.pm.active_bosses.first_mut().unwrap();
        boss.stats.all_stats.get_mut(SPEED).unwrap().current = 10;
        let result = gm
            .pm
            .compute_sup_atk_turn(crate::character::CharacterType::Hero);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn unit_new_round() {
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        gm.start_new_game();
        let result = gm.start_new_turn();
        assert_eq!(result, true);
        assert_eq!(gm.game_state.current_round, 1);
        // TODO add second hero player to test the current round and avoid auto atk of boss
    }

    #[test]
    fn unit_launch_attack_case1() {
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        gm.start_new_game();
        gm.start_new_turn();

        // # case 1 dmg on individual ennemy
        // No dodging of boss
        // no critical of current player
        // TODO load atk by json
        let atk = build_atk_damage1();
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
        let ra = gm.launch_attack(&atk.clone().name);
        assert_eq!(1, ra.outcomes.len());
        assert_eq!(1, ra.all_dodging.len());
        assert_eq!("Boss1", ra.all_dodging[0].name);
        assert_eq!(false, ra.all_dodging[0].is_dodging);
        // 1 dead boss : end of game
        assert_eq!(true, gm.check_end_of_game());
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
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        gm.start_new_game();
        gm.start_new_turn();

        // # case 2 dmg on individual ennemy
        // dodging of boss
        // no critical of current player
        // TODO load atk by json
        let atk = build_atk_damage1();
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
        gm.launch_attack(&atk.clone().name);
        // 1 dead boss : end of game
        assert_eq!(true, gm.check_end_of_game());
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
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        gm.start_new_game();
        gm.start_new_turn();

        // # case 1 dmg on individual ennemy
        // No dodging of boss
        // critical of current player
        // TODO load atk by json
        let atk = build_atk_damage1();
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
        gm.launch_attack(&atk.clone().name);
        // 1 dead boss : end of game
        // assert_eq!(true, gm.check_end_of_game());
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
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        gm.start_new_game();
        gm.start_new_turn();

        // # case 4 dmg on individual ennemy
        // No dodging of boss
        // Blocking
        // No critical of current player
        // TODO load atk by json
        let atk = build_atk_damage1();
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
        gm.launch_attack(&atk.clone().name);
        // 1 dead boss : end of game
        assert_eq!(true, gm.check_end_of_game());
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
    fn integ_dxrpg() {
        let mut gm = GameManager::try_new("offlines").unwrap();
        gm.start_new_game();
        gm.create_game_dirs().unwrap();
        gm.start_new_turn();
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
        let ra = gm.launch_attack("SimpleAtk");
        if ra.all_dodging.len() > 0 && ra.all_dodging[0].is_dodging {
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
            assert_eq!(
                old_hp_boss - 31,
                gm.pm
                    .get_active_boss_character("Angmar")
                    .unwrap()
                    .stats
                    .all_stats[HP]
                    .current
            );
        }
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(2, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(3, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(4, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        assert_eq!(2, gm.game_state.last_result_atks.nb_atk_stored);
        // angmar turn
        /*assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(5, gm.game_state.current_round);
        let ra = gm.launch_attack("SimpleAtk");
         assert_eq!(ra.all_dodging.len() == 1, true);
        assert_eq!(ra.all_dodging[0].name, "Thalia");
        assert_eq!(ra.outcomes.len() > 0, true);
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(6, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        // turn 2
        assert_eq!(2, gm.game_state.current_turn_nb);
        assert_eq!(1, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        assert_eq!(2, gm.game_state.current_turn_nb);
        assert_eq!(2, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        // 2 heroes are dead and their turn index were 3 and 4
        assert_eq!(2, gm.game_state.current_turn_nb);
        assert_eq!(5, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        // one player is dead , round 3 to 5
        assert_eq!(2, gm.game_state.current_turn_nb);
        assert_eq!(6, gm.game_state.current_round);
        // angmar turn
        let _ra = gm.launch_attack("SimpleAtk"); */
        let path = OFFLINE_ROOT.join(paths_const::GAMES_DIR.to_path_buf());
        let big_list = utils::list_dirs_in_dir(path);
        let one_save = big_list.unwrap()[0].clone();
        let _ = gm.load_game(one_save);
        let _ = gm.save_game_manager();
    }
}
