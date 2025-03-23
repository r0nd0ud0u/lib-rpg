use std::{collections::HashMap, path::Path};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    attack_type::AttackType,
    character::{Character, CharacterType},
    common::paths_const::OFFLINE_CHARACTERS,
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
    /*  std::unordered_map<QString, std::vector<GameAtkEffects>>
    m_AllEffectsOnGame; // key target */
    pub all_effects_on_game: HashMap<String, Vec<GameAtkEffects>>,
}

impl PlayerManager {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<PlayerManager> {
        let mut pl = PlayerManager {
            ..Default::default()
        };
        pl.load_all_characters(path)?;
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
            let hp = &mut pl.stats.hp;
            let mana = &mut pl.stats.mana;
            let berseck = &mut pl.stats.berseck;
            let vigor = &mut pl.stats.vigor;
            let speed = &mut pl.stats.speed;

            let regen_hp = &mut pl.stats.hp_regeneration;
            let regen_mana = &mut pl.stats.mana_regeneration;
            let regen_berseck = &mut pl.stats.berseck_rate;
            let regen_vigor = &mut pl.stats.vigor_regeneration;
            let regen_speed = &mut pl.stats.speed_regeneration;

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
            speed.current_raw = speed.max_raw * (speed.current / speed.max);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::players_manager::GameAtkEffects;

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
        let old_hp = pl.all_heroes[0].stats.hp.current;
        let hp_regen = pl.all_heroes[0].stats.hp_regeneration.current;
        let old_mana = pl.all_heroes[0].stats.mana.current;
        let mana_regen = pl.all_heroes[0].stats.mana_regeneration.current;
        let old_berseck = pl.all_heroes[0].stats.berseck.current;
        let berseck_regen = pl.all_heroes[0].stats.berseck_rate.current;
        let old_vigor = pl.all_heroes[0].stats.vigor.current;
        let vigor_regen = pl.all_heroes[0].stats.vigor_regeneration.current;
        let old_speed = pl.all_heroes[0].stats.speed.current;
        let speed_regen = pl.all_heroes[0].stats.speed_regeneration.current;
        pl.apply_regen_stats(crate::character::CharacterType::Hero);
        assert_eq!(old_hp + hp_regen, pl.all_heroes[0].stats.hp.current);
        assert_eq!(
            std::cmp::min(old_mana + mana_regen, pl.all_heroes[0].stats.mana.max),
            pl.all_heroes[0].stats.mana.current
        );
        assert_eq!(
            std::cmp::min(
                old_berseck + berseck_regen,
                pl.all_heroes[0].stats.berseck.max
            ),
            pl.all_heroes[0].stats.berseck.current
        );
        assert_eq!(
            std::cmp::min(old_vigor + vigor_regen, pl.all_heroes[0].stats.vigor.max),
            pl.all_heroes[0].stats.vigor.current
        );
        assert_eq!(
            old_speed + speed_regen,
            pl.all_heroes[0].stats.speed.current
        );

        let old_hp = pl.all_bosses[0].stats.hp.current;
        let hp_regen = pl.all_bosses[0].stats.hp_regeneration.current;
        pl.apply_regen_stats(crate::character::CharacterType::Boss);
        // max is topped
        assert_eq!(
            std::cmp::min(pl.all_bosses[0].stats.hp.max, old_hp + hp_regen),
            pl.all_bosses[0].stats.hp.current
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
}
