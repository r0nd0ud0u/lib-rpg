use crate::{
    character_mod::character::CharacterKind,
    common::{
        constants::stats_const::*,
        log_data::{LogData, const_colors::DARK_RED},
    },
    server::game_state::GameStatus,
};

use super::{GameManager, ResultLaunchAttack};

impl GameManager {
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
        self.pm.start_new_turn(self.game_state.current_turn_nb == 1);

        self.new_round()
    }

    /// Process the order of the players to play by sorting them by speed and adding the supplementary atk turns for the heroes and the bosses
    pub fn process_order_to_play(&mut self) {
        // to be improved with stats
        // one player can play several times as well in different order
        self.game_state.order_to_play.clear();

        // add heroes
        // sort by speed descending (highest speed acts first)
        self.pm
            .active_heroes
            .sort_by(|a, b| b.stats.all_stats[SPEED].cmp(&a.stats.all_stats[SPEED]));
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
        // sort by speed descending (highest speed acts first)
        self.pm
            .active_bosses
            .sort_by(|a, b| b.stats.all_stats[SPEED].cmp(&a.stats.all_stats[SPEED]));
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
        let Ok(mut logs) = self.pm.update_current_player_on_new_round(
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
            let (all_heroes_dead, all_bosses_dead) = self.pm.check_end_of_game();
            if all_heroes_dead {
                self.game_state.status = GameStatus::EndOfGame;
                return (false, logs);
            } else if all_bosses_dead {
                self.game_state.status = GameStatus::EndOfScenario;
                self.process_end_of_scenario();
                return (false, logs);
            }
            return self.new_round();
        }

        self.pm.reset_targeted_character();

        // Insert a round-separator at the front so the log sheet can group events per round
        logs.insert(
            0,
            LogData {
                message: format!(
                    "\u{1f501} Turn {} — Round {}",
                    self.game_state.current_turn_nb, self.game_state.current_round
                ),
                color: crate::common::log_data::const_colors::LIGHT_BLUE.to_owned(),
            },
        );

        (true, logs)
    }

    pub(super) fn process_no_atk_launched(&mut self) -> ResultLaunchAttack {
        // Capture launcher identity before eval_end_of_round() may advance current_player.
        let launcher_id_name = self.pm.current_player.id_name.clone();
        let is_boss_atk = self.pm.current_player.is_boss_atk();
        let turn_nb = self.game_state.current_turn_nb;
        let round_nb = self.game_state.current_round;

        self.pm
            .current_player
            .character_rounds_info
            .actions_done_in_round += 1;
        let logs_atk = vec![LogData {
            message: "No attack launched".to_string(),
            color: DARK_RED.to_string(),
        }];
        let logs_end_of_round = self.eval_end_of_round(logs_atk.clone());
        let result = ResultLaunchAttack {
            launcher_id_name,
            is_boss_atk,
            logs_end_of_round,
            logs_atk,
            turn_nb,
            round_nb,
            ..Default::default()
        };
        // Mirror what launch_attack() does for real attacks so ra.atk_name is
        // empty and the gameboard consumable-action banner branch is reachable.
        self.game_state.last_result_atk = result.clone();
        result
    }

    /// Evaluate the end of the round by checking if the game is finished,
    ///  if a new round should start or if a new turn should start,
    ///  and return the logs to display for the new round if it is the case
    pub(super) fn eval_end_of_round(&mut self, logs_atk: Vec<LogData>) -> Vec<LogData> {
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
            // new_round may have triggered EndOfScenario/EndOfGame (e.g. boss killed by DOT)
            if matches!(
                self.game_state.status,
                GameStatus::EndOfScenario | GameStatus::EndOfGame
            ) {
                // Status already set inside new_round; nothing more to do
            } else if is_new_round {
                self.game_state.status = GameStatus::StartRound;
            } else {
                let (is_new_turn, logs) = self.start_new_turn();
                output_logs.extend(logs);
                if matches!(
                    self.game_state.status,
                    GameStatus::EndOfScenario | GameStatus::EndOfGame
                ) {
                    // Status set inside start_new_turn via new_round
                } else if is_new_turn {
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

    use crate::character_mod::character::CharacterKind;

    use crate::common::constants::{character_const::SPEED_THRESHOLD, stats_const::*};
    use crate::server::game_state::GameStatus;
    use crate::testing::testing_all_characters::{self, testing_game_manager};
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
        // only one supplementary attack per turn: test2_#1 (fastest hero, speed 312) qualifies;
        // test_#1 (212) is skipped because process_sup_atk_turn returns after the first hit.
        assert_eq!(gm.game_state.order_to_play.len(), 5);
        // descending speed sort: test2_#1 (312) before test_#1 (212)
        assert_eq!(gm.game_state.order_to_play[0], "test2_#1");
        assert_eq!(gm.game_state.order_to_play[1], "test_#1");
        // descending speed sort: boss2 (15) before boss1 (11)
        assert_eq!(gm.game_state.order_to_play[2], "test_boss2_#1");
        assert_eq!(gm.game_state.order_to_play[3], "test_boss1_#1");
        // only test2_#1 gets the supplementary slot
        assert_eq!(gm.game_state.order_to_play[4], "test2_#1");
        // test_#1 speed is unchanged (it did NOT get the supplementary slot)
        assert_eq!(old_speed.current, new_speed.current);
        assert_eq!(old_speed.max, new_speed.max);
        assert_eq!(old_speed.max_raw, new_speed.max_raw);
        assert_eq!(old_speed.current_raw, new_speed.current_raw);
        // test2_#1 had its speed reset (312 - SPEED_THRESHOLD = 212)
        let new_test2_speed = gm
            .pm
            .get_mut_active_hero_character("test2_#1")
            .cloned()
            .unwrap()
            .stats
            .all_stats[SPEED]
            .clone();
        assert_eq!(312 - SPEED_THRESHOLD, new_test2_speed.current);
        // one hero player is dead — use name-based kill so the index stays stable after sort
        gm.pm
            .get_mut_active_hero_character("test_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current = 0;
        gm.process_order_to_play();
        assert_eq!(gm.game_state.order_to_play.len(), 5);
        assert_eq!(gm.game_state.order_to_play[0], "test2_#1");
        assert_eq!(gm.game_state.order_to_play[1], "test_#1");
        assert_eq!(gm.game_state.order_to_play[2], "test_boss2_#1");
        assert_eq!(gm.game_state.order_to_play[3], "test_boss1_#1");
        assert_eq!(gm.game_state.order_to_play[4], "test2_#1");
        // boss is dead — use name-based kill; descending sort puts boss2 at index 0
        gm.pm
            .get_mut_active_boss_character("test_boss1_#1")
            .unwrap()
            .stats
            .all_stats[HP]
            .current = 0;
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
        // only one supplementary attack per call — the first qualifying hero
        assert_eq!(result.len(), 1);
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
    fn unit_new_round_all_heroes_dead_end_of_game() {
        let mut gm = testing_game_manager();
        gm.start_game();
        // Kill ALL heroes
        for hero in &mut gm.pm.active_heroes {
            hero.stats.all_stats[HP].current = 0;
        }
        // Make round 1 point to the first hero (who is dead)
        gm.game_state.current_round = 0;
        let (is_new_round, _logs) = gm.new_round();
        assert!(
            !is_new_round,
            "dead player → should not start a new round normally"
        );
        assert_eq!(
            gm.game_state.status,
            GameStatus::EndOfGame,
            "all heroes dead → EndOfGame"
        );
    }
}
