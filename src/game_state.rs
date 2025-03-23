use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    character::CharacterType, common::character_const::SPEED_THRESHOLD,
    players_manager::PlayerManager,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// Current turn number
    pub current_turn_nb: u64,
    /// Key turn number, value name
    pub died_ennemies: HashMap<i32, String>,
    /// List in the ascending order of the players
    pub order_to_play: Vec<String>,
    /// Current round number
    pub current_round: u64,
    /// Name of the game
    pub game_name: String,
    pub pm: PlayerManager,
}

impl GameState {
    pub fn new(pm: PlayerManager) -> Self {
        GameState {
            pm,
            ..Default::default()
        }
    }
    pub fn start_game(&mut self) {
        // create name of exercise
        let time_str = crate::utils::get_current_time_as_string();
        self.game_name = format!("Game_{}", time_str);
    }
    pub fn start_new_turn(&mut self) {
        // Increment turn number
        self.current_turn_nb += 1;
        // Increment round number
        self.current_round += 1;
        // Increment turn effects
        self.pm.increment_counter_effect();
        // Reset new round boolean for characters
        self.pm.reset_is_first_round();
        // Apply regen stats
        self.pm.apply_regen_stats(CharacterType::Boss);
        self.pm.apply_regen_stats(CharacterType::Hero);

        // For each turn now
        // Process the order of the players
        self.process_order_to_play();
    }

    pub fn process_order_to_play(&mut self) {
        // to be improved with stats
        // one player can play several times as well in different order
        self.order_to_play.clear();

        // sort by speed
        self.pm
            .all_heroes
            .sort_by(|a, b| a.stats.speed.cmp(&b.stats.speed));
        let mut dead_heroes = Vec::new();
        for hero in &self.pm.all_heroes {
            if !hero.is_dead() {
                self.order_to_play.push(hero.name.clone());
            } else {
                dead_heroes.push(hero.name.clone());
            }
        }
        // add dead heroes
        for name in dead_heroes {
            self.order_to_play.push(name);
        }
        // add bosses
        // sort by speed
        self.pm
            .all_bosses
            .sort_by(|a, b| a.stats.speed.cmp(&b.stats.speed));
        for boss in &self.pm.all_bosses {
            self.order_to_play.push(boss.name.clone());
        }
        // supplementariy atks to push
        self.add_sup_atk_turn(CharacterType::Hero);
        // self.add_sup_atk_turn(CharacterType::Boss, &mut self.order_to_play);
    }

    pub fn add_sup_atk_turn(&mut self, launcher_type: CharacterType) {
        let (player_list1, player_list2) = if launcher_type == CharacterType::Hero {
            (&mut self.pm.all_heroes, &self.pm.all_bosses)
        } else {
            (&mut self.pm.all_bosses, &self.pm.all_heroes)
        };
        for pl1 in player_list1 {
            if pl1.is_dead() {
                continue;
            }
            let cur_speed_pl1 = pl1.stats.speed.current;
            for pl2 in player_list2 {
                let speed_pl2 = pl2.stats.speed.current;
                if cur_speed_pl1 - speed_pl2 >= SPEED_THRESHOLD {
                    // Update of current value aspeed_threshold
                    pl1.stats.speed.current =
                        pl1.stats.speed.current.saturating_sub(SPEED_THRESHOLD);
                    pl1.stats.speed.max = pl1.stats.speed.max.saturating_sub(SPEED_THRESHOLD);
                    pl1.stats.speed.max_raw =
                        pl1.stats.speed.max_raw.saturating_sub(SPEED_THRESHOLD);
                    pl1.stats.speed.current_raw =
                        pl1.stats.speed.current_raw.saturating_sub(SPEED_THRESHOLD);
                    self.order_to_play.push(pl1.name.clone());
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::character_const::SPEED_THRESHOLD, game_state::GameState,
        players_manager::PlayerManager,
    };

    #[test]
    fn unit_start_game() {
        let mut gs = GameState::default();
        gs.start_game();
        assert!(!gs.game_name.is_empty());
    }

    #[test]
    fn unit_start_new_turn() {
        let mut gs = GameState::default();
        gs.start_new_turn();
        assert_eq!(gs.current_round, 1);
        assert_eq!(gs.current_turn_nb, 1);
    }

    #[test]
    fn unit_process_order_to_play() {
        let mut gs = GameState::new(PlayerManager::try_new("tests/characters").unwrap());
        let old_speed = gs.pm.all_heroes.first().cloned().unwrap().stats.speed;
        gs.process_order_to_play();
        let new_speed = gs.pm.all_heroes.first().cloned().unwrap().stats.speed;
        assert_eq!(gs.order_to_play.len(), 3);
        assert_eq!(gs.order_to_play[0], "Super test");
        assert_eq!(gs.order_to_play[1], "Boss1");
        // supplementary atk
        assert_eq!(gs.order_to_play[2], "Super test");

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
        let mut gs = GameState::new(PlayerManager::try_new("tests/characters").unwrap());
        let hero = gs.pm.all_heroes.first_mut().unwrap();
        hero.stats.speed.current = 300;
        let boss = gs.pm.all_bosses.first_mut().unwrap();
        boss.stats.speed.current = 10;
        gs.add_sup_atk_turn(crate::character::CharacterType::Hero);
        assert_eq!(gs.order_to_play.len(), 1);
    }
}
