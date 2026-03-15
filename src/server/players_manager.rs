use std::collections::HashMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    character_mod::{
        attack_type::AttackType,
        character::{Character, CharacterKind},
        effect::ProcessedEffectParam,
        equipment::{Equipment, EquipmentJsonKey},
    },
    common::{
        constants::{
            all_target_const::{TARGET_ALL_ALLIES, TARGET_ALLY, TARGET_ENNEMY, TARGET_HIMSELF},
            character_const::*,
            reach_const::{INDIVIDUAL, ZONE},
            stats_const::*,
        },
        log_data::LogData,
    },
    server::game_state::GameState,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameAtkEffects {
    pub all_atk_effects: ProcessedEffectParam,
    pub atk: AttackType,
    pub launcher: String,
    pub target: String,
    pub launching_turn: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DodgeInfo {
    pub name: String,
    pub is_dodging: bool,
    pub is_blocking: bool,
}

/// Define all the parameters of a playerManager
/// Should store all the relative data to all the playABLE characters
/// /// Should store all the relative data to all the playING characters
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerManager {
    /// List of all selected heroes by players
    pub active_heroes: Vec<Character>,
    /// List of all selected bosses by computer
    pub active_bosses: Vec<Character>,
    /// Shadow current player used to update the active character in the list of active characters
    pub current_player: Character,
    /// Equipment table mapping character names to their equipped items
    pub equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>,
}

impl PlayerManager {
    /// Try to create a new PlayerManager by loading all the characters
    /// and by initializing the active heroes with all the loaded heroes
    /// if `default_active_characters` is true.
    /// Bosses are always active by default.
    /// `path` is the root path of the offline directory containing characters and equipments directories.
    pub fn new(equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>) -> PlayerManager {
        PlayerManager {
            active_heroes: Vec::new(),
            active_bosses: Vec::new(),
            current_player: Character::default(),
            equipment_table,
        }
    }

    /// Get the number of active heroes with the given name
    pub fn get_nb_of_active_heroes_by_name(&self, db_full_name: &str) -> usize {
        self.active_heroes
            .iter()
            .filter(|c| c.db_full_name == db_full_name)
            .count()
    }

    /// Get the number of active bosses with the given name
    pub fn get_nb_of_active_bosses_by_name(&self, db_full_name: &str) -> usize {
        self.active_bosses
            .iter()
            .filter(|c| c.db_full_name == db_full_name)
            .count()
    }

    pub fn increment_counter_effect(&mut self) {
        for c in self.active_heroes.iter_mut() {
            c.character_rounds_info.increment_counter_effect();
        }
        for c in self.active_bosses.iter_mut() {
            c.character_rounds_info.increment_counter_effect();
        }
    }

    /// The boolean is_first_round is reset for all the characters of the game.
    pub fn reset_is_first_round(&mut self) {
        for c in &mut self.active_heroes {
            c.character_rounds_info.is_first_round = true;
        }
        for c in &mut self.active_bosses {
            c.character_rounds_info.is_first_round = true;
        }
    }

    pub fn apply_regen_stats(&mut self, kind: CharacterKind) {
        let player_list = if kind == CharacterKind::Hero {
            &mut self.active_heroes
        } else {
            &mut self.active_bosses
        };
        for pl in player_list {
            if pl.stats.is_dead().unwrap_or(false) {
                continue;
            }

            pl.stats.apply_regen();
        }
    }

    pub fn get_all_active_id_names(&self) -> Vec<String> {
        let mut output = vec![];
        for h in &self.active_heroes {
            output.push(h.id_name.clone());
        }
        for b in &self.active_bosses {
            output.push(b.id_name.clone());
        }
        output
    }

    pub fn get_mut_active_character(&mut self, id_name: &str) -> Option<&mut Character> {
        if let Some(hero) = self.active_heroes.iter_mut().find(|c| c.id_name == id_name) {
            return Some(hero);
        }
        if let Some(boss) = self.active_bosses.iter_mut().find(|c| c.id_name == id_name) {
            return Some(boss);
        }
        None
    }

    pub fn get_active_character(&self, id_name: &str) -> Option<&Character> {
        if let Some(hero) = self.get_active_hero_character(id_name) {
            return Some(hero);
        }
        if let Some(boss) = self.get_active_boss_character(id_name) {
            return Some(boss);
        }
        None
    }

    pub fn modify_active_character(&mut self, id_name: &str) {
        let pl = self.current_player.clone();
        if let Some(hero) = self.get_mut_active_hero_character(id_name) {
            *hero = pl; // Modify the value inside self.active_heroes
        } else if let Some(boss) = self.get_mut_active_boss_character(id_name) {
            *boss = pl;
        }
    }

    pub fn get_mut_active_hero_character(&mut self, id_name: &str) -> Option<&mut Character> {
        self.active_heroes.iter_mut().find(|c| c.id_name == id_name)
    }

    pub fn get_mut_active_boss_character(&mut self, id_name: &str) -> Option<&mut Character> {
        self.active_bosses.iter_mut().find(|c| c.id_name == id_name)
    }

    pub fn get_active_hero_character(&self, id_name: &str) -> Option<&Character> {
        self.active_heroes.iter().find(|c| c.id_name == id_name)
    }

    pub fn get_active_boss_character(&self, id_name: &str) -> Option<&Character> {
        self.active_bosses.iter().find(|c| c.id_name == id_name)
    }

    pub fn update_current_player_on_new_round(
        &mut self,
        game_state: &GameState,
        id_name: &str,
    ) -> Result<Vec<LogData>> {
        let logs;
        match self.get_mut_active_character(id_name) {
            Some(c) => {
                self.current_player = c.clone();

                // update the shadow current player
                logs = self
                    .current_player
                    .new_round(game_state.current_turn_nb, self.process_launchable_atks());

                // update the active character
                self.modify_active_character(id_name);
                Ok(logs)
            }
            None => {
                bail!("Character '{}' not found", id_name)
            }
        }
    }

    pub fn start_new_turn(&mut self) {
        // Increment turn effects
        self.increment_counter_effect();
        // Reset new round boolean for characters
        self.reset_is_first_round();
        // Apply regen stats
        self.apply_regen_stats(CharacterKind::Boss);
        self.apply_regen_stats(CharacterKind::Hero);
    }

    pub fn process_sup_atk_turn(&mut self, launcher_type: CharacterKind) -> Vec<String> {
        let mut output = Vec::new();
        let (player_list1, player_list2) = if launcher_type == CharacterKind::Hero {
            (&mut self.active_heroes, &self.active_bosses)
        } else {
            (&mut self.active_bosses, &self.active_heroes)
        };
        for pl1 in player_list1 {
            if pl1.stats.is_dead().unwrap_or(false) {
                continue;
            }
            let speed_pl1 = match pl1.stats.all_stats.get(SPEED) {
                Some(speed) => speed,
                None => continue,
            };
            for pl2 in player_list2 {
                let speed_pl2_current = pl2.stats.all_stats[SPEED].current;
                let delta = speed_pl1.current.saturating_sub(speed_pl2_current);
                if delta >= SPEED_THRESHOLD {
                    pl1.stats.reset_speed();
                    output.push(pl1.id_name.clone());
                    break;
                }
            }
        }
        output
    }

    pub fn process_all_dodging(
        &mut self,
        all_targets: &Vec<String>,
        atk_level: u64,
        kind: &CharacterKind,
    ) {
        for t in all_targets {
            match self.get_mut_active_character(t) {
                Some(c) => {
                    if c.kind != *kind {
                        c.process_dodging(atk_level);
                    }
                }
                _ => continue,
            }
        }
    }

    pub fn process_died_players(&mut self) -> Result<()> {
        // heroes
        for c in self.active_heroes.iter_mut() {
            if c.stats.is_dead() == Some(true) {
                c.reset_all_effects_on_player()?; // now ? works
                c.character_rounds_info.reset_all_buffers();
            }
        }
        Ok(())
    }

    pub fn process_boss_target(&mut self) {
        if self.current_player.kind == CharacterKind::Hero {
            return;
        }

        self.reset_targeted_character();
        if let Some((max_index, _)) = self
            .active_heroes
            .iter()
            .enumerate()
            .filter(|(_, c)| c.stats.is_dead() == Some(false))
            .max_by_key(|&(_, c)| c.stats.all_stats[AGGRO].current)
        {
            self.active_heroes[max_index]
                .character_rounds_info
                .is_current_target = true;
        }
    }

    /// Apply target choice from UI
    pub fn set_one_target(&mut self, launcher_id_name: &str, atk_name: &str, target_id_name: &str) {
        if let Some(h) = self.get_mut_active_character(launcher_id_name) {
            let Some(atk) = h.attacks_list.iter().find(|a| a.0 == atk_name) else {
                return;
            };
            if atk.1.reach == ZONE {
                return;
            }
            self.reset_targeted_character();
            if let Some(target) = self.get_mut_active_character(target_id_name) {
                target.character_rounds_info.is_current_target = true;
            }
        }
    }

    /// Get the number of current potential targets (used for UI)
    pub fn get_current_target_nb(&self) -> usize {
        self.active_heroes
            .iter()
            .filter(|c| c.character_rounds_info.is_potential_target)
            .count()
            + self
                .active_bosses
                .iter()
                .filter(|c| c.character_rounds_info.is_potential_target)
                .count()
    }

    pub fn whatif_set_targeted_characters(&self, launcher_id_name: &str, atk_name: &str) -> u64 {
        if let Some(launcher) = self.get_active_character(launcher_id_name) {
            let Some(atk) = launcher
                .attacks_list
                .iter()
                .find(|a| a.0 == atk_name)
                .map(|a| a.1.clone())
            else {
                return 0;
            };

            let is_hero_ally = launcher.kind == CharacterKind::Hero && atk.target == TARGET_ALLY;
            let is_boss_ally = launcher.kind == CharacterKind::Boss && atk.target == TARGET_ALLY;
            let is_boss_ennemy =
                launcher.kind == CharacterKind::Boss && atk.target == TARGET_ENNEMY;
            let is_hero_ennemy =
                launcher.kind == CharacterKind::Hero && atk.target == TARGET_ENNEMY;

            // self - atk
            if atk.target == TARGET_HIMSELF {
                return 1;
            }
            // all heroes - atk
            if atk.target == TARGET_ALL_ALLIES {
                let mut nb = 0;
                self.active_heroes.iter().for_each(|c| {
                    if c.stats.is_dead() == Some(false) {
                        nb += 1;
                    }
                });
                return nb;
            }
            // atk on heroes
            if is_boss_ennemy || is_hero_ally {
                return Self::whatif_targets_for_collection(
                    &self.active_heroes,
                    launcher_id_name,
                    &atk,
                    is_hero_ally,
                    is_boss_ennemy,
                );
            }

            // atk on ennemies
            if is_boss_ally || is_hero_ennemy {
                return Self::whatif_targets_for_collection(
                    &self.active_bosses,
                    launcher_id_name,
                    &atk,
                    is_boss_ally,
                    is_hero_ennemy,
                );
            }
        }

        0
    }

    /// Apply potential target choice for UI
    pub fn set_targeted_characters(&mut self, launcher_id_name: &str, atk_name: &str) {
        self.reset_targeted_character();
        self.reset_potential_targeted_character();

        if let Some(launcher) = self.get_mut_active_character(launcher_id_name) {
            let Some(atk) = launcher
                .attacks_list
                .iter()
                .find(|a| a.0 == atk_name)
                .map(|a| a.1.clone())
            else {
                return;
            };

            let is_hero_ally = launcher.kind == CharacterKind::Hero && atk.target == TARGET_ALLY;
            let is_boss_ally = launcher.kind == CharacterKind::Boss && atk.target == TARGET_ALLY;
            let is_boss_ennemy =
                launcher.kind == CharacterKind::Boss && atk.target == TARGET_ENNEMY;
            let is_hero_ennemy =
                launcher.kind == CharacterKind::Hero && atk.target == TARGET_ENNEMY;

            // self - atk
            if atk.target == TARGET_HIMSELF {
                launcher.character_rounds_info.is_current_target = true;
                launcher.character_rounds_info.is_potential_target = true;
                return;
            }
            // all heroes - atk
            if atk.target == TARGET_ALL_ALLIES {
                self.active_heroes.iter_mut().for_each(|c| {
                    if c.stats.is_dead() == Some(false) {
                        c.character_rounds_info.is_potential_target = true;
                        c.character_rounds_info.is_current_target = true;
                    }
                });
                return;
            }
            // atk on heroes
            if is_boss_ennemy || is_hero_ally {
                Self::set_targets_for_collection(
                    &mut self.active_heroes,
                    launcher_id_name,
                    &atk,
                    is_hero_ally,
                    is_boss_ennemy,
                );
            }

            // atk on ennemies
            if is_boss_ally || is_hero_ennemy {
                Self::set_targets_for_collection(
                    &mut self.active_bosses,
                    launcher_id_name,
                    &atk,
                    is_boss_ally,
                    is_hero_ennemy,
                );
            }
        }
    }

    pub fn reset_targeted_character(&mut self) {
        self.active_heroes
            .iter_mut()
            .for_each(|c| c.character_rounds_info.is_current_target = false);
        self.active_bosses
            .iter_mut()
            .for_each(|c| c.character_rounds_info.is_current_target = false);
    }

    pub fn reset_potential_targeted_character(&mut self) {
        self.active_heroes
            .iter_mut()
            .for_each(|c| c.character_rounds_info.is_potential_target = false);
        self.active_bosses
            .iter_mut()
            .for_each(|c| c.character_rounds_info.is_potential_target = false);
    }

    pub fn process_launchable_atks(&self) -> Vec<AttackType> {
        // assess potential target
        let mut launchable_attacks = Vec::new();

        for atk in self.current_player.attacks_list.values() {
            let can_be_launched = self.current_player.can_be_launched(atk);
            let whatif_nb =
                self.whatif_set_targeted_characters(&self.current_player.id_name, &atk.name);
            if can_be_launched && whatif_nb > 0 {
                launchable_attacks.push(atk.clone());
            }
        }
        launchable_attacks
    }

    /// Helper function to set targets for a given collection of characters
    /// Extracted to avoid code duplication between heroes and bosses targeting
    fn set_targets_for_collection(
        characters: &mut [Character],
        launcher_id_name: &str,
        atk: &AttackType,
        is_ally_condition: bool,
        is_ennemy_condition: bool,
    ) {
        let mut has_at_least_one_target = false;
        characters
            .iter_mut()
            .filter(|c| {
                c.stats.is_dead() == Some(false)
                    && ((is_ally_condition && c.id_name != launcher_id_name) || is_ennemy_condition)
            })
            .for_each(|c| {
                if !has_at_least_one_target && atk.reach == INDIVIDUAL || atk.reach == ZONE {
                    c.character_rounds_info.is_current_target = true;
                    c.character_rounds_info.is_potential_target = true;
                    has_at_least_one_target = true;
                } else {
                    c.character_rounds_info.is_potential_target = true;
                }
            });
    }

    /// Helper function to set targets for a given collection of characters
    /// Extracted to avoid code duplication between heroes and bosses targeting
    fn whatif_targets_for_collection(
        characters: &[Character],
        launcher_id_name: &str,
        atk: &AttackType,
        is_ally_condition: bool,
        is_ennemy_condition: bool,
    ) -> u64 {
        let mut has_at_least_one_target = false;
        let mut nb = 0;
        characters
            .iter()
            .filter(|c| {
                c.stats.is_dead() == Some(false)
                    && ((is_ally_condition && c.id_name != launcher_id_name) || is_ennemy_condition)
            })
            .for_each(|_c| {
                if !has_at_least_one_target && atk.reach == INDIVIDUAL || atk.reach == ZONE {
                    nb += 1;
                    has_at_least_one_target = true;
                } else {
                    nb += 1;
                }
            });
        nb
    }

    pub fn check_end_of_game(&self) -> bool {
        let all_heroes_dead = self
            .active_heroes
            .iter()
            .all(|c| c.stats.is_dead() == Some(true));
        let all_bosses_dead = self
            .active_bosses
            .iter()
            .all(|c| c.stats.is_dead() == Some(true));
        all_bosses_dead || all_heroes_dead
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character_mod::equipment::EquipmentJsonKey,
        common::constants::stats_const::*,
        server::game_state::GameState,
        server::players_manager::GameAtkEffects,
        testing::testing_all_characters::{self, testing_pm},
        testing::testing_effect::*,
    };
    use strum::IntoEnumIterator;

    #[test]
    fn unit_try_new() {
        let pl = testing_all_characters::testing_pm();

        // equipments
        assert_eq!(EquipmentJsonKey::iter().count(), pl.equipment_table.len());
    }

    #[test]
    fn unit_increment_counter_effect() {
        let mut pl = testing_all_characters::testing_pm();
        pl.active_heroes[0]
            .character_rounds_info
            .all_effects
            .push(GameAtkEffects {
                all_atk_effects: build_cooldown_effect(),
                ..Default::default()
            });
        let old_counter_turn = pl.active_heroes[0].character_rounds_info.all_effects[0]
            .all_atk_effects
            .counter_turn;
        pl.increment_counter_effect();
        assert_eq!(
            pl.active_heroes[0].character_rounds_info.all_effects[0]
                .all_atk_effects
                .counter_turn,
            old_counter_turn + 1
        );
    }

    #[test]
    fn unit_reset_is_first_round() {
        let mut pl = testing_all_characters::testing_pm();
        pl.reset_is_first_round();
        assert!(pl.active_heroes[0].character_rounds_info.is_first_round);
    }

    #[test]
    fn unit_apply_regen_stats() {
        let mut pl = testing_all_characters::testing_pm();
        let old_hp = pl.active_heroes[0].stats.all_stats[HP].current;
        let hp_regen = pl.active_heroes[0].stats.all_stats[HP_REGEN].current;
        let old_mana = pl.active_heroes[0].stats.all_stats[MANA].current;
        let mana_regen = pl.active_heroes[0].stats.all_stats[MANA_REGEN].current;
        let old_berseck = pl.active_heroes[0].stats.all_stats[BERSERK].current;
        let berseck_regen = pl.active_heroes[0].stats.all_stats[BERSECK_RATE].current;
        let old_vigor = pl.active_heroes[0].stats.all_stats[VIGOR].current;
        let vigor_regen = pl.active_heroes[0].stats.all_stats[VIGOR_REGEN].current;
        let old_speed = pl.active_heroes[0].stats.all_stats[SPEED].current;
        let speed_regen = pl.active_heroes[0].stats.all_stats[SPEED_REGEN].current;
        pl.apply_regen_stats(crate::character_mod::character::CharacterKind::Hero);
        assert_eq!(
            old_hp + hp_regen,
            pl.active_heroes[0].stats.all_stats[HP].current
        );
        assert_eq!(
            std::cmp::min(
                old_mana + mana_regen,
                pl.active_heroes[0].stats.all_stats[MANA].max
            ),
            pl.active_heroes[0].stats.all_stats[MANA].current
        );
        assert_eq!(
            std::cmp::min(
                old_berseck + berseck_regen,
                pl.active_heroes[0].stats.all_stats[BERSERK].max
            ),
            pl.active_heroes[0].stats.all_stats[BERSERK].current
        );
        assert_eq!(
            std::cmp::min(
                old_vigor + vigor_regen,
                pl.active_heroes[0].stats.all_stats[VIGOR].max
            ),
            pl.active_heroes[0].stats.all_stats[VIGOR].current
        );
        assert_eq!(
            old_speed + speed_regen,
            pl.active_heroes[0].stats.all_stats[SPEED].current
        );

        let old_hp = pl.active_bosses[0].stats.all_stats[HP].current;
        let hp_regen = pl.active_bosses[0].stats.all_stats[HP_REGEN].current;
        pl.apply_regen_stats(crate::character_mod::character::CharacterKind::Boss);
        // max is topped
        assert_eq!(
            std::cmp::min(
                pl.active_bosses[0].stats.all_stats[HP].max,
                old_hp + hp_regen
            ),
            pl.active_bosses[0].stats.all_stats[HP].current
        );
    }

    #[test]
    fn unit_get_mut_active_character() {
        let mut pl = testing_all_characters::testing_pm();
        assert!(pl.get_mut_active_character("test_#1").is_some());
        assert!(pl.get_mut_active_character("test_boss1_#1").is_some());
        assert!(pl.get_mut_active_character("unknown").is_none());
    }

    #[test]
    fn unit_get_active_character() {
        let pl = testing_all_characters::testing_pm();
        assert!(pl.get_active_character("test_#1").is_some());
        assert!(pl.get_active_character("test_boss1_#1").is_some());
        assert!(pl.get_active_character("unknown").is_none());
    }

    #[test]
    fn unit_update_current_player() {
        let mut pl = testing_all_characters::testing_pm();
        pl.get_mut_active_hero_character("test_#1")
            .unwrap()
            .character_rounds_info
            .is_first_round = false;
        pl.get_mut_active_hero_character("test_#1")
            .unwrap()
            .character_rounds_info
            .actions_done_in_round = 100;
        let gs = GameState::default();
        pl.update_current_player_on_new_round(&gs, "test_#1")
            .unwrap();
        assert_eq!(
            0,
            pl.get_mut_active_hero_character("test_#1")
                .unwrap()
                .character_rounds_info
                .actions_done_in_round
        );
    }

    #[test]
    fn unit_pl_process_hot_and_dot() {
        let mut pl = testing_all_characters::testing_pm();
        // push default effect
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffects::default());
        let mut gs = GameState::new();
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // test cooldown effect
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffects {
                all_atk_effects: build_cooldown_effect(),
                ..Default::default()
            });
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // add test HOT but on same turn
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffects {
                all_atk_effects: build_hot_effect_individual(),
                ..Default::default()
            });
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // add test HOT on different turn
        gs.start_new_turn();
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(1, logs.len());
        assert_eq!(30, hot_and_dot);
        // add test DOT on different turn
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffects {
                all_atk_effects: build_dot_effect_individual(),
                ..Default::default()
            });
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(2, logs.len()); // hot + dot
        assert_eq!(10, hot_and_dot); // 30(hot) - 20 (dot)
    }

    #[test]
    fn unit_set_one_target() {
        let mut pl = testing_all_characters::testing_pm();
        // simpleAtk is indiv launched by a boss
        pl.set_one_target("test_boss1_#1", "SimpleAtk", "test_#1");
        assert!(
            pl.get_mut_active_hero_character("test_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_mut_active_boss_character("test_boss1_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        // indiv launched a hero
        pl.set_one_target("test_#1", "SimpleAtk", "test_boss1_#1");
        assert!(
            !pl.get_mut_active_hero_character("test_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_mut_active_boss_character("test_boss1_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        // whatever launched with ZONE no reset is done
        pl.set_one_target("test_#1", "simple-atk-zone", "test_boss1_#1");
        assert!(
            !pl.get_mut_active_hero_character("test_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_mut_active_boss_character("test_boss1_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
        pl.set_one_target("test_#1", "Offrande vitale", "test2_#1");
        assert!(
            pl.get_mut_active_hero_character("test2_#1")
                .unwrap()
                .character_rounds_info
                .is_current_target
        );
    }

    #[test]
    fn unit_set_targeted_characters() {
        let mut pl = testing_pm();
        // hero is attacking
        // atk to ennemy - effect dmg indiv
        let test_ally_id_name = "test_#1";
        let test2_ally_id_name = "test2_#1";
        let boss_id_name = "test_boss1_#1";
        let boss2_id_name = "test_boss2_#1";
        pl.get_active_character(test_ally_id_name).expect("no hero");
        pl.set_targeted_characters(test_ally_id_name, "SimpleAtk");
        assert_eq!(2, pl.active_bosses.len());
        let current_nb = pl
            .active_bosses
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_current_target)
            .count();
        assert_eq!(current_nb, 1);
        let potential_nb = pl
            .active_bosses
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_potential_target)
            .count();
        assert_eq!(potential_nb, 2);
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ennemy - effect dmg zone
        pl.set_targeted_characters(test_ally_id_name, "simple-atk-zone");
        assert!(
            pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal indiv, test -> test2
        pl.set_targeted_characters(test_ally_id_name, "simple-atk-himself");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target,
        );
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target,
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal indiv, test2 -> test
        pl.set_targeted_characters(test2_ally_id_name, "simple-atk-himself");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ally - effect heal zone
        pl.set_targeted_characters(test_ally_id_name, "simple-atk-ally-zone");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(test2_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to all heroes target
        pl.set_targeted_characters(test_ally_id_name, "simple-atk-all-heroes");
        let current_nb = pl
            .active_heroes
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_current_target)
            .count();
        assert_eq!(current_nb, 2);
        let potential_nb = pl
            .active_heroes
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_potential_target)
            .count();
        assert_eq!(potential_nb, 2);

        // boss is attacking
        // atk from ennemy - effect dmg indiv
        pl.set_targeted_characters(boss_id_name, "SimpleAtk");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        let nb = pl
            .active_heroes
            .iter_mut()
            .filter(|x| x.character_rounds_info.is_current_target)
            .count();
        assert_eq!(nb, 1);
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target,
        );
        // atk from ennemy - effect dmg zone
        pl.set_targeted_characters(boss_id_name, "simple-atk-zone");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal indiv
        pl.set_targeted_characters(boss_id_name, "simple-atk-himself");
        assert!(
            pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
        // boss atk to ally - effect heal zone  => ZONE is not himself
        pl.set_targeted_characters(boss_id_name, "simple-atk-ally-zone");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            pl.get_active_character(boss2_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss2_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_id_name)
                .expect("no hero")
                .character_rounds_info
                .is_potential_target
        );
    }

    #[test]
    fn unit_set_targeted_characters_test_dead_character() {
        let mut pl = testing_pm();
        // hero is attacking
        // atk to ennemy - effect dmg indiv
        let test_ally_id_name = "test_#1";
        let boss_id_name = "test_boss1_#1";
        let boss2_id_name = "test_boss2_#1";
        pl.get_active_character(test_ally_id_name).expect("no hero");
        pl.get_mut_active_character(boss_id_name)
            .expect("no boss")
            .stats
            .all_stats
            .get_mut(HP)
            .unwrap()
            .current = 0; // boss is dead
        pl.set_targeted_characters(test_ally_id_name, "SimpleAtk");
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
        // consequently only one boss remaining, that boss is the target
        assert!(
            pl.get_active_character(boss2_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss2_id_name)
                .expect("no boss")
                .character_rounds_info
                .is_potential_target
        );
    }

    #[test]
    fn unit_get_nb_of_active_heroes_by_name() {
        let pl = testing_all_characters::testing_pm();
        assert_eq!(1, pl.get_nb_of_active_heroes_by_name("test"));
        assert_eq!(0, pl.get_nb_of_active_heroes_by_name("unknown"));
    }

    #[test]
    fn unit_get_nb_of_active_bosses_by_name() {
        let pl = testing_all_characters::testing_pm();
        assert_eq!(1, pl.get_nb_of_active_bosses_by_name("test_boss1"));
        assert_eq!(0, pl.get_nb_of_active_bosses_by_name("unknown"));
    }

    #[test]
    fn unit_get_current_target_nb() {
        let mut pl = testing_all_characters::testing_pm();
        assert_eq!(0, pl.get_current_target_nb());
        pl.active_heroes[0]
            .character_rounds_info
            .is_potential_target = true;
        assert_eq!(1, pl.get_current_target_nb());
        pl.active_bosses[0]
            .character_rounds_info
            .is_potential_target = true;
        assert_eq!(2, pl.get_current_target_nb());
    }

    #[test]
    fn unit_whatif_set_targeted_characters() {
        let pl = testing_all_characters::testing_pm();
        // hero is attacking
        // atk to ennemy - effect dmg indiv
        let test_ally_id_name = "test_#1";
        pl.get_active_character(test_ally_id_name).expect("no hero");
        let potential_target_nb = pl.whatif_set_targeted_characters(test_ally_id_name, "SimpleAtk");
        assert_eq!(2, potential_target_nb);
        // atk to ennemy - effect dmg zone
        let potential_target_nb =
            pl.whatif_set_targeted_characters(test_ally_id_name, "simple-atk-zone");
        assert_eq!(2, potential_target_nb);
        // atk to ally(himself in this example) - effect heal indiv, test -> test2
        let potential_target_nb =
            pl.whatif_set_targeted_characters(test_ally_id_name, "simple-atk-himself");
        assert_eq!(1, potential_target_nb);
    }

    #[test]
    fn unit_process_launchable_atks() {
        let mut pl = testing_all_characters::testing_pm();
        // no problem of level
        pl.current_player.level = 100;
        // no problem of is_heal_atk_blocked
        pl.current_player.character_rounds_info.is_heal_atk_blocked = false;
        let launchable_atks = pl.process_launchable_atks();
        // print launcgable atks for debug
        //println!("Launchable attacks: {:?}", launchable_atks);
        assert_eq!(pl.current_player.attacks_list.len(), launchable_atks.len());

        // case level under
        pl.current_player.level = 1;
        let launchable_atks = pl.process_launchable_atks();
        assert_eq!(12, launchable_atks.len()); // 12 on 16 are level 1

        // case is_heal_atk_blocked
        pl.current_player.character_rounds_info.is_heal_atk_blocked = true;
        pl.current_player.level = 100;
        let launchable_atks = pl.process_launchable_atks();
        assert_eq!(10, launchable_atks.len()); // 6 attacks are HP and linked to is_heal_atk_blocked condition
    }
}
