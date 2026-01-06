use std::path::Path;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    attack_type::AttackType,
    character::{AmountType, Character, CharacterType},
    common::{
        all_target_const::{TARGET_ALLY, TARGET_ENNEMY, TARGET_HIMSELF},
        character_const::*,
        paths_const::OFFLINE_CHARACTERS,
        reach_const::{INDIVIDUAL, ZONE},
        stats_const::*,
    },
    effect::{is_effet_hot_or_dot, EffectParam},
    game_state::GameState,
    utils::list_files_in_dir,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameAtkEffects {
    pub all_atk_effects: EffectParam,
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
    /// List of all playable heroes -> player
    pub all_heroes: Vec<Character>,
    /// List of all playable bosses -> computer
    pub all_bosses: Vec<Character>,
    /// List of all selected heroes by players
    pub active_heroes: Vec<Character>,
    /// List of all selected bosses by computer
    pub active_bosses: Vec<Character>,
    pub current_player: Character,
}

impl PlayerManager {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<PlayerManager> {
        let mut pl = PlayerManager {
            all_heroes: Vec::new(),
            all_bosses: Vec::new(),
            active_heroes: Vec::new(),
            active_bosses: Vec::new(),
            current_player: Character::default(),
        };
        pl.load_all_characters(path)?;
        pl.active_heroes = pl.all_heroes.clone();
        pl.active_bosses = pl.all_bosses.clone();
        Ok(pl)
    }

    pub fn testing_pm() -> PlayerManager {
        let mut pl = PlayerManager::try_new("tests/offlines").unwrap();
        pl.current_player = pl.active_heroes[0].clone();
        pl
    }

    /// Load all the JSON files in a path `P` which corresponds to a directory.
    /// Characters are inserted in Hero or Boss lists.
    pub fn load_all_characters<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        if path.as_ref().as_os_str().is_empty() {
            bail!("no root path")
        }
        let character_dir_path = path.as_ref().join(*OFFLINE_CHARACTERS);
        match list_files_in_dir(&character_dir_path) {
            Ok(list) => list.iter().for_each(|character_path| {
                match Character::try_new_from_json(character_path, path.as_ref(), false) {
                    Ok(c) => {
                        if c.kind == CharacterType::Hero {
                            self.all_heroes.push(c);
                        } else {
                            self.all_bosses.push(c);
                        }
                    }
                    Err(e) => println!("{:?} cannot be decoded: {}", character_path, e),
                }
            }),
            Err(e) => bail!("Files cannot be listed in {:#?}: {}", character_dir_path, e),
        };
        Ok(())
    }

    /// Characters are inserted in Hero or Boss lists.
    pub fn load_active_characters_from_saved_game<P: AsRef<Path>>(
        &mut self,
        root_path: P,
    ) -> Result<()> {
        if root_path.as_ref().as_os_str().is_empty() {
            bail!("no root path")
        }
        self.active_heroes.clear();
        self.active_bosses.clear();
        // Load characters from the directory
        let character_dir_path = root_path.as_ref().join(*OFFLINE_CHARACTERS);
        match list_files_in_dir(&character_dir_path) {
            Ok(list) => list.iter().for_each(|character_path| {
                match Character::try_new_from_json(character_path, root_path.as_ref(), true) {
                    Ok(c) => {
                        if c.kind == CharacterType::Hero {
                            self.active_heroes.push(c);
                        } else {
                            self.active_bosses.push(c);
                        }
                    }
                    Err(e) => println!("{:?} cannot be decoded: {}", character_path, e),
                }
            }),
            Err(e) => bail!("Files cannot be listed in {:#?}: {}", character_dir_path, e),
        };
        Ok(())
    }

    pub fn increment_counter_effect(&mut self) {
        for c in self.active_heroes.iter_mut() {
            c.increment_counter_effect();
        }
        for c in self.active_bosses.iter_mut() {
            c.increment_counter_effect();
        }
    }

    /*
     * @brief PlayersManager::ResetIsFirstRound
     * The boolean is_first_round is reset for all the characters of the game.
     */
    pub fn reset_is_first_round(&mut self) {
        for c in &mut self.active_heroes {
            c.extended_character.is_first_round = true;
        }
        for c in &mut self.active_bosses {
            c.extended_character.is_first_round = true;
        }
    }

    // TODO change swap remove see processCost
    pub fn apply_regen_stats(&mut self, kind: CharacterType) {
        let player_list = if kind == CharacterType::Hero {
            &mut self.all_heroes
        } else {
            &mut self.all_bosses
        };
        for pl in player_list {
            if pl.is_dead().unwrap_or(false) {
                continue;
            }

            // TODO change swap remove see processCost
            let mut hp = pl.stats.all_stats.swap_remove(HP).expect("hp is missing");
            let mut mana = pl
                .stats
                .all_stats
                .swap_remove(MANA)
                .expect("mana is missing");
            let mut berseck = pl
                .stats
                .all_stats
                .swap_remove(BERSERK)
                .expect("berseck is missing");
            let mut vigor = pl
                .stats
                .all_stats
                .swap_remove(VIGOR)
                .expect("vigor is missing");
            let mut speed = pl
                .stats
                .all_stats
                .swap_remove(SPEED)
                .expect("speed is missing");

            let regen_hp = &pl.stats.all_stats[HP_REGEN];
            let regen_mana = &pl.stats.all_stats[MANA_REGEN];
            let regen_berseck = &pl.stats.all_stats[BERSECK_RATE];
            let regen_vigor = &pl.stats.all_stats[VIGOR_REGEN];
            let regen_speed = &pl.stats.all_stats[SPEED_REGEN];

            hp.current = std::cmp::min(hp.max, hp.current + regen_hp.current);
            hp.current_raw = hp.max_raw * (hp.current / hp.max);

            mana.current = std::cmp::min(mana.max, mana.current + regen_mana.current);
            if mana.max > 0 {
                mana.current_raw = mana.max_raw * (mana.current / mana.max);
            }

            vigor.current = std::cmp::min(vigor.max, vigor.current + regen_vigor.current);
            if vigor.max > 0 {
                vigor.current_raw = vigor.max_raw * (vigor.current / vigor.max);
            }

            berseck.current = std::cmp::min(berseck.max, berseck.current + regen_berseck.current);
            if berseck.max > 0 {
                berseck.max_raw = berseck.current_raw * (berseck.current / berseck.max);
            }

            speed.current += regen_speed.current;
            speed.max += regen_speed.current;
            speed.max_raw += regen_speed.current;
            // TODO change current raw calculation
            if speed.max > 0 {
                speed.current_raw = speed.max_raw * (speed.current / speed.max);
            }

            pl.stats.all_stats.insert(HP.to_owned(), hp);
            pl.stats.all_stats.insert(MANA.to_owned(), mana);
            pl.stats.all_stats.insert(VIGOR.to_owned(), vigor);
            pl.stats.all_stats.insert(SPEED.to_owned(), speed);
            pl.stats.all_stats.insert(BERSERK.to_owned(), berseck);
        }
    }

    pub fn get_all_active_names(&self) -> Vec<String> {
        let mut output = vec![];
        for h in &self.active_heroes {
            output.push(h.name.clone());
        }
        for b in &self.active_bosses {
            output.push(b.name.clone());
        }
        output
    }

    pub fn get_mut_active_character(&mut self, name: &str) -> Option<&mut Character> {
        if let Some(hero) = self.active_heroes.iter_mut().find(|c| c.name == name) {
            return Some(hero);
        }
        if let Some(boss) = self.active_bosses.iter_mut().find(|c| c.name == name) {
            return Some(boss);
        }
        None
    }

    pub fn get_active_character(&self, name: &str) -> Option<&Character> {
        if let Some(hero) = self.get_active_hero_character(name) {
            return Some(hero);
        }
        if let Some(boss) = self.get_active_boss_character(name) {
            return Some(boss);
        }
        None
    }

    pub fn modify_active_character(&mut self, name: &str) {
        let pl = self.current_player.clone();
        if let Some(hero) = self.get_mut_active_hero_character(name) {
            *hero = pl; // Modify the value inside self.active_heroes
        } else if let Some(boss) = self.get_mut_active_boss_character(name) {
            *boss = pl;
        }
    }

    pub fn get_mut_active_hero_character(&mut self, name: &str) -> Option<&mut Character> {
        self.active_heroes.iter_mut().find(|c| c.name == name)
    }

    pub fn get_mut_active_boss_character(&mut self, name: &str) -> Option<&mut Character> {
        self.active_bosses.iter_mut().find(|c| c.name == name)
    }

    pub fn get_active_hero_character(&self, name: &str) -> Option<&Character> {
        self.active_heroes.iter().find(|c| c.name == name)
    }

    pub fn get_active_boss_character(&self, name: &str) -> Option<&Character> {
        self.active_bosses.iter().find(|c| c.name == name)
    }

    pub fn update_current_player(
        &mut self,
        game_state: &GameState,
        name: &str,
    ) -> Result<Vec<String>> {
        let mut logs = Vec::new();
        match self.get_mut_active_character(name) {
            Some(c) => {
                self.current_player = c.clone();

                // update the shadow current player
                self.current_player.actions_done_in_round = 0;

                if self.current_player.extended_character.is_first_round {
                    self.current_player.extended_character.is_first_round = false;
                    // aggro is initialized before any action
                    self.current_player
                        .init_aggro_on_turn(game_state.current_turn_nb);
                    let _ = self
                        .current_player
                        .remove_terminated_effect_on_player()
                        .iter()
                        .map(|e| {
                            logs.push(format!("{} on {}", e.effect_type, e.stats_name));
                        });
                    // TODO apply passive power

                    // apply hot and dot
                    let (mut process_logs, hot_or_dot) = self.process_hot_and_dot(game_state);
                    self.apply_hot_or_dot(game_state, hot_or_dot);

                    //self.apply_all_effects_on_player(game_state, false);
                    // process logs
                    logs.append(&mut process_logs);
                }

                // update the active character
                self.modify_active_character(name);
                Ok(logs)
            }
            None => {
                bail!("Character '{}' not found", name)
            }
        }
    }

    fn apply_hot_or_dot(&mut self, game_state: &GameState, hot_or_dot: i64) {
        if hot_or_dot != 0 {
            let hp = self.current_player.stats.all_stats.get_mut(HP).unwrap();
            if hot_or_dot < 0 {
                hp.current = hp.current.saturating_sub(hot_or_dot.unsigned_abs());
            } else {
                hp.current = hp.current.saturating_add(hot_or_dot as u64);
            }

            // localLog.append(QString("HOT et DOT totaux: %1").arg(hotAndDot));
            // update buf overheal
            let delta_over_heal: i64 = hp.current as i64 - hp.max as i64;
            if delta_over_heal > 0 {
                // update txrx
                self.current_player.tx_rx[AmountType::OverHealRx as usize]
                    .insert(game_state.current_turn_nb as u64, delta_over_heal);
            }
            // current value must be included between 0 and max value
            hp.current = std::cmp::min(hp.current, hp.max);
            hp.current = std::cmp::max(hp.current, 0);
        }
    }

    pub fn process_hot_and_dot(&mut self, game_state: &GameState) -> (Vec<String>, i64) {
        let mut logs = Vec::new();
        let mut hot_and_dot = 0;
        // First process all the effects whatever their order
        for gae in self.current_player.all_effects.iter() {
            if gae.launching_turn == game_state.current_turn_nb {
                continue;
            }
            // Process hot or dot
            if gae.all_atk_effects.stats_name == HP
                && is_effet_hot_or_dot(&gae.all_atk_effects.effect_type)
            {
                process_hot_or_dot(&mut logs, &mut hot_and_dot, gae);
            }
        }
        (logs, hot_and_dot)
    }

    pub fn start_new_turn(&mut self) {
        // Increment turn effects
        self.increment_counter_effect();
        // Reset new round boolean for characters
        self.reset_is_first_round();
        // Apply regen stats
        self.apply_regen_stats(CharacterType::Boss);
        self.apply_regen_stats(CharacterType::Hero);
    }

    pub fn compute_sup_atk_turn(&mut self, launcher_type: CharacterType) -> Vec<String> {
        let mut output = Vec::new();
        let (player_list1, player_list2) = if launcher_type == CharacterType::Hero {
            (&mut self.active_heroes, &self.active_bosses)
        } else {
            (&mut self.active_bosses, &self.active_heroes)
        };
        for pl1 in player_list1 {
            if pl1.is_dead().unwrap_or(false) {
                continue;
            }
            let speed_pl1 = match pl1.stats.all_stats.get_mut(SPEED) {
                Some(speed) => speed,
                None => continue,
            };
            for pl2 in player_list2 {
                let speed_pl2_current = pl2.stats.all_stats[SPEED].current;
                let delta = speed_pl1.current.saturating_sub(speed_pl2_current);
                if delta >= SPEED_THRESHOLD {
                    // Update of current value aspeed_threshold
                    speed_pl1.current = speed_pl1.current.saturating_sub(SPEED_THRESHOLD);
                    speed_pl1.max = speed_pl1.max.saturating_sub(SPEED_THRESHOLD);
                    speed_pl1.max_raw = speed_pl1.max_raw.saturating_sub(SPEED_THRESHOLD);
                    speed_pl1.current_raw = speed_pl1.current_raw.saturating_sub(SPEED_THRESHOLD);
                    output.push(pl1.name.clone());
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
        kind: &CharacterType,
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

    pub fn process_died_players(&mut self) {
        // heroes
        self.active_heroes.iter_mut().for_each(|c| {
            if c.is_dead() == Some(true) {
                c.reset_all_effects_on_player();
                c.reset_all_buffers();
            }
        });
    }

    pub fn process_boss_target(&mut self) {
        if self.current_player.kind == CharacterType::Hero {
            return;
        }

        self.reset_targeted_character();
        if let Some((max_index, _)) = self
            .active_heroes
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_dead() == Some(false))
            .max_by_key(|&(_, c)| c.stats.all_stats[AGGRO].current)
        {
            self.active_heroes[max_index].is_current_target = true;
        }
    }

    pub fn set_one_target(&mut self, launcher_name: &str, atk_name: &str, target_name: &str) {
        if let Some(h) = self.get_mut_active_character(launcher_name) {
            let Some(atk) = h.attacks_list.iter().find(|a| a.0 == atk_name) else {
                return;
            };
            if atk.1.reach == ZONE {
                return;
            }
            self.reset_targeted_character();
            if let Some(target) = self.get_mut_active_character(target_name) {
                target.is_current_target = true;
            }
        }
    }

    // TODO test with a second boss!!!
    pub fn set_targeted_characters(&mut self, launcher_name: &str, atk_name: &str) {
        self.reset_targeted_character();
        self.reset_potential_targeted_character();

        if let Some(launcher) = self.get_mut_active_character(launcher_name) {
            let Some(atk) = launcher
                .attacks_list
                .iter()
                .find(|a| a.0 == atk_name)
                .map(|a| a.1.clone())
            else {
                return;
            };

            let is_hero_ally = launcher.kind == CharacterType::Hero && atk.target == TARGET_ALLY;
            let is_boss_ally = launcher.kind == CharacterType::Boss && atk.target == TARGET_ALLY;
            let is_boss_ennemy =
                launcher.kind == CharacterType::Boss && atk.target == TARGET_ENNEMY;
            let is_hero_ennemy =
                launcher.kind == CharacterType::Hero && atk.target == TARGET_ENNEMY;

            // self - atk
            if atk.target == TARGET_HIMSELF {
                launcher.is_current_target = true;
                launcher.is_potential_target = true;
                return;
            }
            // individual atk on an hero
            if (is_boss_ennemy || is_hero_ally) && atk.reach == INDIVIDUAL {
                if is_boss_ennemy {
                    // default behavior - auto atk expected so that those are not useful
                    if let Some(c) = self.active_heroes.first_mut() {
                        c.is_current_target = true;
                        c.is_potential_target = true;
                    }
                    self.active_heroes
                        .iter_mut()
                        .for_each(|c| c.is_potential_target = true);
                } else {
                    if let Some(item) = self
                        .active_heroes
                        .iter_mut()
                        .find(|x| x.name != launcher_name)
                    {
                        item.is_current_target = true;
                    }
                    self.active_heroes
                        .iter_mut()
                        .filter(|x| x.name == launcher_name)
                        .for_each(|c| c.is_potential_target = true);
                }
            }

            // individual atk on an ennemy
            if (is_boss_ally || is_hero_ennemy) && atk.reach == INDIVIDUAL {
                if is_hero_ennemy {
                    if let Some(c) = self.active_bosses.first_mut() {
                        c.is_current_target = true;
                        c.is_potential_target = true
                    }
                    self.active_bosses
                        .iter_mut()
                        .filter(|x| x.name == launcher_name)
                        .for_each(|c| c.is_potential_target = true);
                } else {
                    if let Some(item) = self
                        .active_bosses
                        .iter_mut()
                        .find(|x| x.name != launcher_name)
                    {
                        item.is_current_target = true;
                    }
                    self.active_bosses
                        .iter_mut()
                        .for_each(|c| c.is_potential_target = true);
                }
            }
            // Zone atk
            if (is_boss_ennemy || is_hero_ally) && atk.reach == ZONE {
                self.active_heroes
                    .iter_mut()
                    .for_each(|c| c.is_current_target = true);
            }
            if (is_boss_ally || is_hero_ennemy) && atk.reach == ZONE {
                self.active_bosses
                    .iter_mut()
                    .for_each(|c| c.is_current_target = true);
            }
        }
    }

    pub fn reset_targeted_character(&mut self) {
        self.active_heroes
            .iter_mut()
            .for_each(|c| c.is_current_target = false);
        self.active_bosses
            .iter_mut()
            .for_each(|c| c.is_current_target = false);
    }

    pub fn reset_potential_targeted_character(&mut self) {
        self.active_heroes
            .iter_mut()
            .for_each(|c| c.is_potential_target = false);
        self.active_bosses
            .iter_mut()
            .for_each(|c| c.is_potential_target = false);
    }
}

fn process_hot_or_dot(local_log: &mut Vec<String>, hot_and_dot: &mut i64, gae: &GameAtkEffects) {
    *hot_and_dot += gae.all_atk_effects.value;
    let effect_type = if gae.all_atk_effects.value > 0 {
        "HOT->"
    } else {
        "DOT->"
    };
    local_log.push(format!(
        "{} valeur: {}, atk: {}",
        effect_type, gae.all_atk_effects.value, gae.atk.name
    ));
}

#[cfg(test)]
mod tests {
    use crate::{
        common::stats_const::*, game_state::GameState, players_manager::GameAtkEffects,
        testing_effect::*,
    };

    use super::PlayerManager;

    #[test]
    fn unit_try_new() {
        let pl = PlayerManager::try_new("tests/offlines").unwrap();
        assert_eq!(2, pl.all_heroes.len());

        assert!(PlayerManager::try_new("").is_err());
    }

    #[test]
    fn unit_increment_counter_effect() {
        let mut pl = PlayerManager::try_new("tests/offlines").unwrap();
        pl.active_heroes[0].all_effects.push(GameAtkEffects {
            all_atk_effects: build_cooldown_effect(),
            ..Default::default()
        });
        let old_counter_turn = pl.active_heroes[0].all_effects[0]
            .all_atk_effects
            .counter_turn;
        pl.increment_counter_effect();
        assert_eq!(
            pl.active_heroes[0].all_effects[0]
                .all_atk_effects
                .counter_turn,
            old_counter_turn + 1
        );
    }

    #[test]
    fn unit_reset_is_first_round() {
        let mut pl = PlayerManager::try_new("tests/offlines").unwrap();
        pl.reset_is_first_round();
        assert!(pl.all_heroes[0].extended_character.is_first_round);
    }

    #[test]
    fn unit_apply_regen_stats() {
        let mut pl = PlayerManager::try_new("tests/offlines").unwrap();
        let old_hp = pl.all_heroes[0].stats.all_stats[HP].current;
        let hp_regen = pl.all_heroes[0].stats.all_stats[HP_REGEN].current;
        let old_mana = pl.all_heroes[0].stats.all_stats[MANA].current;
        let mana_regen = pl.all_heroes[0].stats.all_stats[MANA_REGEN].current;
        let old_berseck = pl.all_heroes[0].stats.all_stats[BERSERK].current;
        let berseck_regen = pl.all_heroes[0].stats.all_stats[BERSECK_RATE].current;
        let old_vigor = pl.all_heroes[0].stats.all_stats[VIGOR].current;
        let vigor_regen = pl.all_heroes[0].stats.all_stats[VIGOR_REGEN].current;
        let old_speed = pl.all_heroes[0].stats.all_stats[SPEED].current;
        let speed_regen = pl.all_heroes[0].stats.all_stats[SPEED_REGEN].current;
        pl.apply_regen_stats(crate::character::CharacterType::Hero);
        assert_eq!(
            old_hp + hp_regen,
            pl.all_heroes[0].stats.all_stats[HP].current
        );
        assert_eq!(
            std::cmp::min(
                old_mana + mana_regen,
                pl.all_heroes[0].stats.all_stats[MANA].max
            ),
            pl.all_heroes[0].stats.all_stats[MANA].current
        );
        assert_eq!(
            std::cmp::min(
                old_berseck + berseck_regen,
                pl.all_heroes[0].stats.all_stats[BERSERK].max
            ),
            pl.all_heroes[0].stats.all_stats[BERSERK].current
        );
        assert_eq!(
            std::cmp::min(
                old_vigor + vigor_regen,
                pl.all_heroes[0].stats.all_stats[VIGOR].max
            ),
            pl.all_heroes[0].stats.all_stats[VIGOR].current
        );
        assert_eq!(
            old_speed + speed_regen,
            pl.all_heroes[0].stats.all_stats[SPEED].current
        );

        let old_hp = pl.all_bosses[0].stats.all_stats[HP].current;
        let hp_regen = pl.all_bosses[0].stats.all_stats[HP_REGEN].current;
        pl.apply_regen_stats(crate::character::CharacterType::Boss);
        // max is topped
        assert_eq!(
            std::cmp::min(pl.all_bosses[0].stats.all_stats[HP].max, old_hp + hp_regen),
            pl.all_bosses[0].stats.all_stats[HP].current
        );
    }

    #[test]
    fn unit_load_all_characters() {
        let mut pl = PlayerManager::default();
        pl.load_all_characters("tests/offlines").unwrap();
        assert_eq!(2, pl.all_heroes.len());
    }

    #[test]
    fn unit_load_all_characters_err() {
        let mut pl = PlayerManager::default();
        assert!(pl.load_all_characters("").is_err());
    }

    #[test]
    fn unit_get_mut_active_character() {
        let mut pl = PlayerManager::try_new("tests/offlines").unwrap();
        assert!(pl.get_mut_active_character("test").is_some());
        assert!(pl.get_mut_active_character("Boss1").is_some());
        assert!(pl.get_mut_active_character("unknown").is_none());
    }

    #[test]
    fn unit_get_active_character() {
        let pl = PlayerManager::try_new("tests/offlines").unwrap();
        assert!(pl.get_active_character("test").is_some());
        assert!(pl.get_active_character("Boss1").is_some());
        assert!(pl.get_active_character("unknown").is_none());
    }

    #[test]
    fn unit_update_current_player() {
        let mut pl = PlayerManager::testing_pm();
        pl.get_mut_active_hero_character("test")
            .unwrap()
            .extended_character
            .is_first_round = false;
        pl.get_mut_active_hero_character("test")
            .unwrap()
            .actions_done_in_round = 100;
        let gs = GameState::default();
        pl.update_current_player(&gs, "test").unwrap();
        assert_eq!(
            0,
            pl.get_mut_active_hero_character("test")
                .unwrap()
                .actions_done_in_round
        );
    }

    #[test]
    fn unit_process_hot_and_dot() {
        let mut pl = PlayerManager::testing_pm();
        // push default effect
        pl.current_player
            .all_effects
            .push(GameAtkEffects::default());
        let mut gs = GameState::new();
        let (logs, hot_and_dot) = pl.process_hot_and_dot(&gs);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // test cooldown effect
        pl.current_player.all_effects.push(GameAtkEffects {
            all_atk_effects: build_cooldown_effect(),
            ..Default::default()
        });
        let (logs, hot_and_dot) = pl.process_hot_and_dot(&gs);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // add test HOT but on same turn
        pl.current_player.all_effects.push(GameAtkEffects {
            all_atk_effects: build_hot_effect_individual(),
            ..Default::default()
        });
        let (logs, hot_and_dot) = pl.process_hot_and_dot(&gs);
        assert_eq!(0, logs.len());
        assert_eq!(0, hot_and_dot);
        // add test HOT on different turn
        let _ = gs.start_new_turn();
        let (logs, hot_and_dot) = pl.process_hot_and_dot(&gs);
        assert_eq!(1, logs.len());
        assert_eq!(30, hot_and_dot);
        // add test DOT on different turn
        pl.current_player.all_effects.push(GameAtkEffects {
            all_atk_effects: build_dot_effect_individual(),
            ..Default::default()
        });
        let (logs, hot_and_dot) = pl.process_hot_and_dot(&gs);
        assert_eq!(2, logs.len()); // hot + dot
        assert_eq!(10, hot_and_dot); // 30(hot) - 20 (dot)
    }

    #[test]
    fn unit_apply_hot_or_dot() {
        let mut pl = PlayerManager::testing_pm();
        let gs = GameState::default();
        pl.current_player.stats.all_stats[HP].current = 100;
        pl.current_player.stats.all_stats[HP].max = 100;
        pl.current_player.stats.all_stats[HP].max_raw = 100;
        pl.current_player.stats.all_stats[HP].current_raw = 100;
        // max value is topped, 100 and not 100 + 30
        pl.apply_hot_or_dot(&gs, 30);
        assert_eq!(100, pl.current_player.stats.all_stats[HP].current);

        pl.apply_hot_or_dot(&gs, -30);
        assert_eq!(70, pl.current_player.stats.all_stats[HP].current);
    }

    #[test]
    fn unit_load_active_characters_from_saved_game() {
        let mut pl = PlayerManager::testing_pm();
        let result = pl.load_active_characters_from_saved_game("");
        assert!(result.is_err());
        let result = pl.load_active_characters_from_saved_game("unknown");
        assert!(result.is_err());
        let file_path = "./tests/offlines/"; // Path to the JSON file
        let result = pl.load_active_characters_from_saved_game(file_path);
        assert!(result.is_ok());
        assert_eq!(2, pl.active_heroes.len());
        assert_eq!(1, pl.active_bosses.len());
        // we are not loading for a save game
        // atks are not loaded from atk files
        assert_eq!(true, pl.active_heroes[0].attacks_list.is_empty());
    }

    #[test]
    fn unit_set_one_target() {
        let mut pl = PlayerManager::testing_pm();
        // simpleAtk is indiv launched by a boss
        pl.set_one_target("Boss1", "SimpleAtk", "test");
        assert!(
            pl.get_mut_active_hero_character("test")
                .unwrap()
                .is_current_target
        );
        assert!(
            !pl.get_mut_active_boss_character("Boss1")
                .unwrap()
                .is_current_target
        );
        // indiv launched a hero
        pl.set_one_target("test", "SimpleAtk", "Boss1");
        assert!(
            !pl.get_mut_active_hero_character("test")
                .unwrap()
                .is_current_target
        );
        assert!(
            pl.get_mut_active_boss_character("Boss1")
                .unwrap()
                .is_current_target
        );
        // whatever launched with ZONE no reset is done
        pl.set_one_target("test", "simple-atk-zone", "Boss1");
        assert_eq!(
            false,
            pl.get_mut_active_hero_character("test")
                .unwrap()
                .is_current_target
        );
        assert_eq!(
            true,
            pl.get_mut_active_boss_character("Boss1")
                .unwrap()
                .is_current_target
        );
    }

    #[test]
    fn unit_set_targeted_characters() {
        let mut pl = PlayerManager::testing_pm();
        // hero is attacking
        // atk to ennemy - effect dmg indiv
        let test_ally_name = "test";
        let test2_ally_name = "test2";
        let boss_name = "Boss1";
        pl.get_active_character(test_ally_name).expect("no hero");
        pl.set_targeted_characters(test_ally_name, "SimpleAtk");
        assert!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target
        );
        assert!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_potential_target
        );
        // atk to ennemy - effect dmg zone
        pl.set_targeted_characters(test_ally_name, "simple-atk-zone");
        assert!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal indiv, test -> test2
        pl.set_targeted_characters(test_ally_name, "simple-atk-himself");
        assert!(
            !pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target,
        );
        assert!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target,
        );
        assert!(
            !pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal indiv, test2 -> test
        pl.set_targeted_characters(test2_ally_name, "simple-atk-himself");
        assert!(
            !pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_potential_target
        );
        assert!(
            !pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target
        );
        // atk to ally(himself in this example) - effect heal zone  => ZONE is not himself
        pl.set_targeted_characters(test_ally_name, "simple-atk-ally-zone");
        assert!(
            !pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target
        );
        assert!(
            pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_current_target
        );
        assert!(
            !pl.get_active_character(test2_ally_name)
                .expect("no hero")
                .is_potential_target
        );

        // boss is attacking
        // atk from ennemy - effect dmg indiv
        pl.set_targeted_characters(boss_name, "SimpleAtk");
        assert_eq!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target,
            false
        );
        assert_eq!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target,
            false
        );
        let nb = pl
            .active_heroes
            .iter_mut()
            .filter(|x| x.is_current_target)
            .count();
        assert_eq!(nb, 1);
        assert_eq!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target,
            true
        );
        // atk from ennemy - effect dmg zone
        pl.set_targeted_characters(boss_name, "simple-atk-zone");
        assert_eq!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target,
            false
        );
        assert_eq!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target,
            false
        );
        assert_eq!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_current_target,
            true
        );
        assert_eq!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target,
            false
        );
        // atk to ally(himself in this example) - effect heal indiv
        pl.set_targeted_characters(boss_name, "simple-atk-himself");
        assert_eq!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target,
            true
        );
        assert_eq!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target,
            true
        );
        assert_eq!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_current_target,
            false
        );
        assert_eq!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target,
            false
        );
        // atk to ally(himself in this example) - effect heal zone  => ZONE is not himself
        pl.set_targeted_characters(boss_name, "simple-atk-ally-zone");
        assert_eq!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_current_target,
            true
        );
        assert_eq!(
            pl.get_active_character(boss_name)
                .expect("no boss")
                .is_potential_target,
            false
        );
        assert_eq!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_current_target,
            false
        );
        assert_eq!(
            pl.get_active_character(test_ally_name)
                .expect("no hero")
                .is_potential_target,
            false
        );
    }
}
