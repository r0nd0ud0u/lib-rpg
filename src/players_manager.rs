use std::{collections::HashMap, path::Path};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    attack_type::AttackType,
    character::{Character, CharacterType},
    common::{paths_const::OFFLINE_CHARACTERS, stats_const::*},
    effect::EffectParam,
    utils::list_files_in_dir,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameAtkEffects {
    pub all_atk_effects: EffectParam,
    pub atk: AttackType,
    pub launcher: String,
    pub target: String,
    pub launching_turn: i32,
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
    /// key target character name, value: all the effects on the game
    pub all_effects_on_game: HashMap<String, Vec<GameAtkEffects>>,
    pub current_player: Character,
}

impl PlayerManager {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<PlayerManager> {
        let mut pl = PlayerManager {
            ..Default::default()
        };
        pl.load_all_characters(path)?;
        pl.active_heroes = pl.all_heroes.clone();
        pl.active_bosses = pl.all_bosses.clone();
        Ok(pl)
    }

    /// Load all the JSON files in a path `P` which corresponds to a directory.
    /// Characters are inserted in Hero or Boss lists.
    pub fn load_all_characters<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        match list_files_in_dir(&path) {
            Ok(list) => list
                .iter()
                .for_each(|path| match Character::try_new_from_json(path) {
                    Ok(c) => {
                        if c.kind == CharacterType::Hero {
                            self.all_heroes.push(c);
                        } else {
                            self.all_bosses.push(c);
                        }
                    }
                    Err(e) => println!("{:?} cannot be decoded: {}", path, e),
                }),
            Err(e) => bail!(
                "Files cannot be listed in {:#?}: {}",
                OFFLINE_CHARACTERS.as_os_str(),
                e
            ),
        };
        Ok(())
    }

    pub fn increment_counter_effect(&mut self) {
        for gae_table in self.all_effects_on_game.values_mut() {
            for gae in gae_table {
                gae.all_atk_effects.counter_turn += 1;
            }
        }
    }

    /*
     * @brief PlayersManager::ResetIsFirstRound
     * The boolean is_first_round is reset for all the characters of the game.
     */
    pub fn reset_is_first_round(&mut self) {
        for c in &mut self.all_heroes {
            c.extended_character.is_first_round = true;
        }
        for c in &mut self.all_bosses {
            c.extended_character.is_first_round = true;
        }
    }

    pub fn apply_regen_stats(&mut self, kind: CharacterType) {
        let player_list = if kind == CharacterType::Hero {
            &mut self.all_heroes
        } else {
            &mut self.all_bosses
        };
        for pl in player_list {
            if pl.is_dead() {
                continue;
            }

            let mut hp = pl.stats.all_stats.remove(HP).expect("hp is missing");
            let mut mana = pl.stats.all_stats.remove(MANA).expect("mana is missing");
            let mut berseck = pl
                .stats
                .all_stats
                .remove(BERSECK)
                .expect("berseck is missing");
            let mut vigor = pl.stats.all_stats.remove(VIGOR).expect("vigor is missing");
            let mut speed = pl.stats.all_stats.remove(SPEED).expect("speed is missing");

            let regen_hp = &pl.stats.all_stats[HP_REGEN];
            let regen_mana = &pl.stats.all_stats[MANA_REGEN];
            let regen_berseck = &pl.stats.all_stats[BERSECK_RATE];
            let regen_vigor = &pl.stats.all_stats[VIGOR_REGEN];
            let regen_speed = &pl.stats.all_stats[SPEED_REGEN];

            hp.current = std::cmp::min(hp.max, hp.current + regen_hp.current);
            hp.current_raw = hp.max_raw * (hp.current / hp.max);

            mana.current = std::cmp::min(mana.max, mana.current + regen_mana.current);
            mana.current_raw = mana.max_raw * (mana.current / mana.max);

            vigor.current = std::cmp::min(vigor.max, vigor.current + regen_vigor.current);
            vigor.current_raw = vigor.max_raw * (vigor.current / vigor.max);

            berseck.current = std::cmp::min(berseck.max, berseck.current + regen_berseck.current);
            berseck.max_raw = berseck.current_raw * (berseck.current / berseck.max);

            speed.current += regen_speed.current;
            speed.max += regen_speed.current;
            speed.max_raw += regen_speed.current;
            // TODO change current raw calculation
            speed.current_raw = speed.max_raw * (speed.current / speed.max);

            pl.stats.all_stats.insert(HP.to_owned(), hp);
            pl.stats.all_stats.insert(MANA.to_owned(), mana);
            pl.stats.all_stats.insert(VIGOR.to_owned(), vigor);
            pl.stats.all_stats.insert(SPEED.to_owned(), speed);
            pl.stats.all_stats.insert(BERSECK.to_owned(), berseck);
        }
    }

    pub fn get_active_character(&mut self, name: &str) -> Option<&mut Character> {
        if let Some(hero) = self.active_heroes.iter_mut().find(|c| c.name == name) {
            return Some(hero);
        }
        if let Some(boss) = self.active_bosses.iter_mut().find(|c| c.name == name) {
            return Some(boss);
        }
        None
    }

    pub fn modify_active_character(&mut self, name: &str) {
        if let Some(hero) = self.active_heroes.iter_mut().find(|c| c.name == name) {
            *hero = self.current_player.clone(); // Modify the value inside self.active_heroes
        }
        if let Some(boss) = self.active_bosses.iter_mut().find(|c| c.name == name) {
            *boss = self.current_player.clone();
        }
    }

    pub fn get_active_hero_character(&mut self, name: &str) -> Option<&mut Character> {
        self.active_heroes.iter_mut().find(|c| c.name == name)
    }

    pub fn get_active_boss_character(&mut self, name: &str) -> Option<&mut Character> {
        self.active_bosses.iter_mut().find(|c| c.name == name)
    }
    pub fn update_current_player(&mut self, current_turn: u64, name: &str) -> Result<()> {
        let c = self
            .get_active_character(name)
            .expect("no active character");
        self.current_player = c.clone();

        // update the shadow current player
        self.current_player.actions_done_in_round = 0;

        if self.current_player.extended_character.is_first_round {
            self.current_player.extended_character.is_first_round = false;
            self.current_player.init_aggro_on_turn(current_turn);
            self.remove_terminated_effect_on_player()?;
        }

        // update the active character
        self.modify_active_character(name);
        Ok(())
    }

    pub fn remove_terminated_effect_on_player(&mut self) -> Result<()> {
        for gae in self.all_effects_on_game[&self.current_player.name].clone() {
            if gae.all_atk_effects.counter_turn == gae.all_atk_effects.nb_turns {
                // TODO add log: effect is terminated
                self.current_player
                    .remove_malus_effect(&gae.all_atk_effects);
            }
        }
        self.all_effects_on_game
            .get_mut(&self.current_player.name)
            .expect("no effect")
            .retain(|element| {
                element.all_atk_effects.nb_turns != element.all_atk_effects.counter_turn
            });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{common::stats_const::*, players_manager::GameAtkEffects};

    use super::PlayerManager;

    #[test]
    fn unit_try_new() {
        let pl = PlayerManager::try_new("tests/characters").unwrap();
        assert_eq!(1, pl.all_heroes.len());

        assert!(PlayerManager::try_new("").is_err());
    }

    #[test]
    fn unit_increment_counter_effect() {
        let mut pl = PlayerManager::try_new("tests/characters").unwrap();
        let mut gaes = Vec::new();
        gaes.push(GameAtkEffects::default());
        pl.all_effects_on_game.insert("target".to_string(), gaes);
        pl.increment_counter_effect();
        assert_eq!(1, pl.all_effects_on_game.get("target").unwrap().len());
    }

    #[test]
    fn unit_reset_is_first_round() {
        let mut pl = PlayerManager::try_new("tests/characters").unwrap();
        pl.reset_is_first_round();
        assert!(pl.all_heroes[0].extended_character.is_first_round);
    }

    #[test]
    fn unit_apply_regen_stats() {
        let mut pl = PlayerManager::try_new("tests/characters").unwrap();
        let old_hp = pl.all_heroes[0].stats.all_stats[HP].current;
        let hp_regen = pl.all_heroes[0].stats.all_stats[HP_REGEN].current;
        let old_mana = pl.all_heroes[0].stats.all_stats[MANA].current;
        let mana_regen = pl.all_heroes[0].stats.all_stats[MANA_REGEN].current;
        let old_berseck = pl.all_heroes[0].stats.all_stats[BERSECK].current;
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
                pl.all_heroes[0].stats.all_stats[BERSECK].max
            ),
            pl.all_heroes[0].stats.all_stats[BERSECK].current
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
        pl.load_all_characters("tests/characters").unwrap();
        assert_eq!(1, pl.all_heroes.len());
    }

    #[test]
    fn unit_load_all_characters_err() {
        let mut pl = PlayerManager::default();
        assert!(pl.load_all_characters("").is_err());
    }

    #[test]
    fn unit_get_active_character() {
        let mut pl = PlayerManager::try_new("tests/characters").unwrap();
        assert!(pl.get_active_character("Super test").is_some());
        assert!(pl.get_active_character("Boss1").is_some());
        assert!(pl.get_active_character("unknown").is_none());
    }

    #[test]
    fn unit_update_current_player() {
        let mut pl = PlayerManager::try_new("tests/characters").unwrap();
        pl.active_heroes[0].extended_character.is_first_round = false;
        pl.active_heroes[0].actions_done_in_round = 100;
        pl.update_current_player(1, "Super test").unwrap();
        assert_eq!(0, pl.active_heroes[0].actions_done_in_round);
        //assert_eq!(0, pl.current_player.actions_done_in_round);
    }
}
