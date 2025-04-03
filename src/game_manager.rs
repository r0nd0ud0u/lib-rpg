use std::path::Path;

use crate::{
    character::{AmountType, CharacterType},
    common::{all_target_const::TARGET_ENNEMY, paths_const::OFFLINE_CHARACTERS, stats_const::*},
    game_state::GameState,
    players_manager::PlayerManager,
    target::TargetInfo,
};
use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};

/// The entry of the library.
/// That object should be called to access to all the different functionalities.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManager {
    pub game_state: GameState,
    /// Player manager
    pub pm: PlayerManager,
}

impl GameManager {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<GameManager> {
        let mut new_path = path.as_ref();
        if new_path.as_os_str().is_empty() {
            new_path = &OFFLINE_CHARACTERS;
        }
        let pm = PlayerManager::try_new(new_path)?;
        Ok(GameManager {
            game_state: GameState::new(),
            pm,
        })
    }
    pub fn start_game(&mut self) {
        self.game_state.init();
    }
    pub fn start_new_turn(&mut self) -> Result<()> {
        // For each turn now
        // Process the order of the players
        self.process_order_to_play();

        self.game_state.start_new_turn()?;

        self.new_round()?;

        // TODO update game status
        // TODO init target view
        // TODO add channel for the logs

        Ok(())
    }

    pub fn process_order_to_play(&mut self) {
        // to be improved with stats
        // one player can play several times as well in different order
        self.game_state.order_to_play.clear();

        // add heroes
        // sort by speed
        self.pm
            .all_heroes
            .sort_by(|a, b| a.stats.all_stats[SPEED].cmp(&b.stats.all_stats[SPEED]));
        let mut dead_heroes = Vec::new();
        for hero in &self.pm.all_heroes {
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
            .all_bosses
            .sort_by(|a, b| a.stats.all_stats[SPEED].cmp(&b.stats.all_stats[SPEED]));
        for boss in &self.pm.all_bosses {
            self.game_state.order_to_play.push(boss.name.clone());
        }
        // supplementary atks to be added
        let supp_rounds_heroes = self.pm.compute_sup_atk_turn(CharacterType::Hero);
        let supp_rounds_bosses = self.pm.compute_sup_atk_turn(CharacterType::Boss);
        self.game_state.order_to_play.extend(supp_rounds_heroes);
        self.game_state.order_to_play.extend(supp_rounds_bosses);
    }

    pub fn new_round(&mut self) -> Result<()> {
        self.game_state.new_round();

        // Still round to play
        if self.game_state.current_round == self.game_state.order_to_play.len() {
            return Ok(());
        }
        self.pm.update_current_player(
            &self.game_state,
            &self.game_state.order_to_play[self.game_state.current_round],
        )?;

        // Those 2 TODO are logs to give info
        // TODO case BOSS: random atk to choose
        // TODO who has the most aggro ?

        // TODO update game status
        // TODO channels for logss

        Ok(())
    }

    /**
     * @brief GameDisplay::LaunchAttak
     * Atk of the launcher is processed first to enable the potential bufs
     * then the effets are processed on the other targets(ennemy and allies)
     */
    pub fn launch_attack(&mut self, atk_name: &str, all_targets: Vec<TargetInfo>) {
        self.pm.current_player.actions_done_in_round += 1;
        if !self.pm.current_player.attacks_list.contains_key(atk_name) {
            // TODO log
            return;
        }

        if !self.pm.current_player.attacks_list.contains_key(atk_name) {
            // TODO log
            return;
        }
        self.pm.current_player.process_atk_cost(atk_name);

        // is dodging ?
        let _is_dodging = if self.pm.current_player.attacks_list[atk_name].target == TARGET_ENNEMY {
            self.pm.is_dodging(
                &all_targets,
                self.pm.current_player.attacks_list[atk_name].level.into(),
            )
        } else {
            vec![]
        };

        // critical strike
        let is_crit = self.pm.current_player.process_critical_strike(atk_name);
        // TODO ProcessIsRandomTarget

        // ProcessAtk
        let atk_list = self.pm.current_player.attacks_list.clone();
        let atk = if let Some(atk) = atk_list.get(atk_name) {
            atk
        } else {
            return;
        };
        let all_effects_param = self
            .pm
            .current_player
            .process_atk(&self.game_state, is_crit, atk);
        let launcher_stats = self.pm.current_player.stats.clone();
        let name = self.pm.current_player.name.clone();
        let kind = self.pm.current_player.kind.clone();
        for ep in &all_effects_param {
            for target in &all_targets {
                if let Some(c) = self.pm.get_mut_active_character(&target.name) {
                    if c.is_targeted(ep, &name, &kind, target.is_targeted) {
                        c.apply_effect_outcome(ep, &launcher_stats, is_crit);
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
        // if boss -> loot
        // handle end of game if all bosses are dead
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::{character_const::SPEED_THRESHOLD, stats_const::SPEED},
        game_manager::GameManager,
        testing_atk::build_atk_damage1,
    };

    use crate::testing_target::build_target_boss_indiv;

    #[test]
    fn unit_try_new() {
        // if empty path, should use the default path
        let gm = GameManager::try_new("").unwrap();
        assert_eq!(gm.pm.all_heroes.len(), 2);

        assert!(GameManager::try_new("unknown").is_err());

        let gm = GameManager::try_new("./tests/characters").unwrap();
        assert_eq!(gm.pm.all_heroes.len(), 1);
    }

    #[test]
    fn unit_process_order_to_play() {
        let mut gm = GameManager::try_new("./tests/characters").unwrap();
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
        assert_eq!(gm.game_state.order_to_play[0], "Super test");
        assert_eq!(gm.game_state.order_to_play[1], "Boss1");
        // supplementary atk
        assert_eq!(gm.game_state.order_to_play[2], "Super test");

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
        let mut gm = GameManager::try_new("./tests/characters").unwrap();
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
        let mut gm = GameManager::try_new("./tests/characters").unwrap();
        gm.start_game();
        gm.start_new_turn().unwrap();
        assert_eq!(gm.game_state.current_round, 1);
        gm.new_round().unwrap();
        assert_eq!(gm.game_state.current_round, 2);
    }

    #[test]
    fn unit_launch_attack() {
        let mut gm = GameManager::try_new("./tests/characters").unwrap();
        gm.start_game();
        gm.start_new_turn().unwrap();
        gm.launch_attack(&build_atk_damage1().name, vec![build_target_boss_indiv()]);
        assert_eq!(gm.pm.current_player.actions_done_in_round, 1);

        // TODO check cost

        // TODO check
    }
}
