use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    character::{AmountType, CharacterType},
    common::{paths_const::*, stats_const::*},
    effect::EffectOutcome,
    game_state::{GameState, GameStatus},
    players_manager::{DodgeInfo, GameAtkEffects, PlayerManager},
    utils,
};
use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultLaunchAttack {
    pub launcher_name: String,
    pub outcomes: Vec<EffectOutcome>,
    pub is_crit: bool,
    pub all_dodging: Vec<DodgeInfo>,
    pub is_auto_atk: bool,
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
        self.pm
            .load_active_characters_from_saved_game(&game_path_dir)?;
        Ok(())
    }

    pub fn start_new_turn(&mut self) -> bool {
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

    pub fn new_round(&mut self) -> bool {
        self.game_state.new_round();
        // Still round to play
        if self.game_state.current_round > self.game_state.order_to_play.len() {
            return false;
        }
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

        true
    }

    pub fn is_auto_atk(&self) -> bool {
        self.pm.current_player.kind == CharacterType::Boss
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
        // is atk existing?
        let atk_list = self.pm.current_player.attacks_list.clone();
        let atk = if let Some(atk) = atk_list.get(atk_name) {
            atk
        } else {
            return ResultLaunchAttack::default();
        };
        // process cost
        self.pm.current_player.process_atk_cost(atk_name);

        // is dodging ?
        self.pm.process_all_dodging(
            &all_players,
            self.pm.current_player.attacks_list[atk_name].level.into(),
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
                        // update all effects
                        c.all_effects.push(GameAtkEffects {
                            all_atk_effects: ep.clone(),
                            atk: atk.clone(),
                            launcher: name.clone(),
                            target: "".to_owned(),
                            launching_turn: self.game_state.current_turn_nb,
                        });
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
            is_auto_atk: self.is_auto_atk(),
        };
        if self.check_end_of_game() {
            self.game_state.status = GameStatus::EndOfGame;
        } else if self.new_round() {
            self.game_state.status = GameStatus::StartRound;
        } else {
            self.start_new_turn();
            self.game_state.status = GameStatus::StartRound;
        }

        self.game_state.last_result_atk = result_attack.clone();

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
}

#[cfg(test)]
mod tests {
    use crate::character::Class;
    use crate::common::attak_const::COEFF_CRIT_DMG;
    use crate::common::paths_const::{self, OFFLINE_ROOT};
    use crate::game_manager::ResultLaunchAttack;
    use crate::game_state::GameStatus;
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
        assert_eq!(gm.pm.all_heroes.len(), 2);

        assert!(GameManager::try_new("unknown").is_err());

        // offline_root by default
        let gm = GameManager::try_new("").unwrap();
        assert_eq!(gm.pm.all_heroes.len(), 4);
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
        assert_eq!(gm.game_state.order_to_play.len(), 5);
        assert_eq!(gm.game_state.order_to_play[0], "test");
        assert_eq!(gm.game_state.order_to_play[1], "test2");
        assert_eq!(gm.game_state.order_to_play[2], "Boss1");
        // supplementary atk
        assert_eq!(gm.game_state.order_to_play[3], "test");
        assert_eq!(gm.game_state.order_to_play[4], "test2");
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
        assert_eq!(gm.game_state.order_to_play.len(), 4);
        assert_eq!(gm.game_state.order_to_play[0], "test2");
        assert_eq!(gm.game_state.order_to_play[1], "test");
        assert_eq!(gm.game_state.order_to_play[2], "Boss1");
        assert_eq!(gm.game_state.order_to_play[3], "test2");
        // boss is dead
        gm.pm.active_bosses[0].stats.all_stats[HP].current = 0;
        gm.process_order_to_play();
        assert_eq!(gm.game_state.order_to_play.len(), 3);
        assert_eq!(gm.game_state.order_to_play[0], "test2");
        assert_eq!(gm.game_state.order_to_play[1], "test");
        assert_eq!(gm.game_state.order_to_play[2], "test2");
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
        // there are 2 allies in the test/offlines to len = 2
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn unit_new_round() {
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        gm.start_new_game();
        let result = gm.start_new_turn();
        assert_eq!(result, true);
        assert_eq!(gm.game_state.current_round, 1);
        // TODO add second hero player to test the current round and avoid auto atk of boss

        // test current player -test- is dead - round for boss is starting
        gm.game_state.current_round = 0;
        gm.pm.active_heroes[0].stats.all_stats[HP].current = 0;
        let result = gm.new_round();
        assert_eq!(true, result);
        assert_eq!(gm.game_state.current_round, 2);
        // test current round > table order to play
        gm.game_state.current_round = 1000;
        let result = gm.new_round();
        assert_eq!(false, result);
        // character name in orderToplay list is not a player
        gm.game_state.order_to_play.clear();
        gm.game_state.order_to_play.push("unknown".to_owned());
        gm.game_state.current_round = 0;
        let result = gm.new_round();
        assert_eq!(false, result);
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
        let ra = gm.launch_attack("");
        assert_eq!(ResultLaunchAttack::default(), ra);
        // test normal atk
        let ra = gm.launch_attack(&atk.clone().name);
        assert_eq!(1, ra.outcomes.len());
        assert_eq!(1, ra.all_dodging.len());
        assert_eq!("Boss1", ra.all_dodging[0].name);
        assert_eq!(false, ra.all_dodging[0].is_dodging);
        // not dead boss : end of game
        assert_eq!(false, gm.check_end_of_game());
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
        gm.launch_attack(&atk.clone().name);
        // not dead boss : end of game
        assert_eq!(false, gm.check_end_of_game());
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
        gm.launch_attack(&atk.clone().name);
        // not dead boss : end of game
        assert_eq!(false, gm.check_end_of_game());
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
        let mut gm = GameManager::try_new("./tests/offlines").unwrap();
        gm.start_new_game();
        gm.start_new_turn();

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
        gm.launch_attack(&atk.clone().name);
        assert_eq!(false, gm.check_end_of_game());
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
    fn unit_integ_dxrpg() {
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
        let _ra = gm.launch_attack("SimpleAtk");
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(3, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(4, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk");
        assert_eq!(gm.check_end_of_game(), false);
        assert_eq!(GameStatus::StartRound, gm.game_state.status);
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(5, gm.game_state.current_round);
        // check if a boss is auto playing
        assert_eq!(true, gm.is_round_auto());
        let _ra = gm.launch_attack("SimpleAtk"); // one hero could be dead
        assert_eq!(gm.check_end_of_game(), false);
        assert_eq!(GameStatus::StartRound, gm.game_state.status);
        assert_eq!(1, gm.game_state.current_turn_nb);
        assert_eq!(6, gm.game_state.current_round);
        let _ra = gm.launch_attack("SimpleAtk"); // one hero could be dead
        assert_eq!(gm.check_end_of_game(), false);
        assert_eq!(GameStatus::StartRound, gm.game_state.status);
        assert_eq!(2, gm.game_state.current_turn_nb);
        assert_eq!(1, gm.game_state.current_round);
        // ensure there is no dead lock -> game can be ended
        while gm.game_state.status == GameStatus::StartRound {
            let _ra = gm.launch_attack("SimpleAtk");
        }
        assert_eq!(GameStatus::EndOfGame, gm.game_state.status);

        // check save game
        let path = OFFLINE_ROOT.join(paths_const::GAMES_DIR.to_path_buf());
        let big_list = utils::list_dirs_in_dir(path);
        let one_save = big_list.unwrap()[0].clone();
        let result = gm.load_game("");
        assert_eq!(true, result.is_err());
        let _ = gm.load_game(one_save);
        let _ = gm.save_game_manager();
    }
}
