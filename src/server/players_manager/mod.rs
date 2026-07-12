use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    character_mod::{
        character::{Character, CharacterKind},
        equipment::{Equipment, EquipmentJsonKey},
        inventory::Consumable,
        talent::TalentTree,
    },
    common::constants::stats_const::*,
};

mod accessors;
mod consumables;
mod game_atk_effect;
mod targeting;
mod turn_flow;

pub use game_atk_effect::GameAtkEffect;

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
    /// Full roster of all bosses loaded from the data manager.
    /// Used as a source when populating active_bosses for each new scenario.
    pub all_bosses: Vec<Character>,
    /// Shadow current player used to update the active character in the list of active characters
    pub current_player: Character,
    /// Equipment table mapping character names to their equipped items
    pub equipment_table: HashMap<EquipmentJsonKey, Vec<Equipment>>,
    /// Shared party consumables pool — available to any hero, consumed when used
    #[serde(default)]
    pub party_consumables: Vec<Consumable>,
    /// Talent tree definitions, keyed by hero `db_full_name`. Static content, sent
    /// to clients alongside `equipment_table` so the Talents sheet can render tiers,
    /// costs and descriptions without a separate round-trip.
    #[serde(default)]
    pub talent_trees: HashMap<String, TalentTree>,
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
            all_bosses: Vec::new(),
            current_player: Character::default(),
            equipment_table,
            party_consumables: Vec::new(),
            talent_trees: HashMap::new(),
        }
    }

    pub fn clear_scenario(&mut self) {
        self.active_bosses.clear();
        self.current_player = Character::default();
        self.active_heroes.iter_mut().for_each(|c| {
            // Reverse active ChangeMaxStat* effects before clearing so buf_effect_*
            // fields are reset to zero and the next scenario starts from the correct base.
            c.reset_all_effects_on_player()
                .expect("failed to reset all effects");
            // Clamp any stat inflated above its max by uncapped passive boosts
            // (e.g. OverHealBoostStat writes directly to stat.current without a cap).
            for stat in c.stats.all_stats.values_mut() {
                stat.current = stat.current.min(stat.max);
            }
            c.character_rounds_info.clear();
            c.stats.get_mut_value(HP).current = c.stats.all_stats[HP].max;
            c.stats.get_mut_value(MANA).current = c.stats.all_stats[MANA].max;
            c.stats.get_mut_value(VIGOR).current = c.stats.all_stats[VIGOR].max;
            c.stats.get_mut_value(BERSERK).current = 0;
            c.stats.get_mut_value(SPEED).current = 0;
            // Reset displayed aggro so the new scenario starts from 0.
            if let Some(aggro) = c.stats.all_stats.get_mut(AGGRO) {
                aggro.current = 0;
            }
        });
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
}

#[cfg(test)]
mod tests {
    use crate::{
        character_mod::equipment::EquipmentJsonKey,
        common::constants::stats_const::*,
        server::players_manager::GameAtkEffect,
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
    fn unit_all_bosses() {
        use crate::character_mod::character::CharacterKind;
        let pl = testing_all_characters::testing_pm();
        assert!(!pl.all_bosses.is_empty(), "all_bosses should not be empty");
        for b in &pl.all_bosses {
            assert_eq!(
                b.kind,
                CharacterKind::Boss,
                "all_bosses should only contain Boss characters, got {:?}",
                b.db_full_name
            );
        }
    }

    #[test]
    fn unit_increment_counter_effect() {
        let mut pl = testing_all_characters::testing_pm();
        pl.active_heroes[0]
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: build_cooldown_effect(),
                ..Default::default()
            });
        let old_counter_turn = pl.active_heroes[0].character_rounds_info.all_effects[0]
            .processed_effect_param
            .counter_turn;
        pl.increment_counter_effect();
        assert_eq!(
            pl.active_heroes[0].character_rounds_info.all_effects[0]
                .processed_effect_param
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
    fn unit_pl_process_hot_and_dot() {
        use crate::character_mod::effect::EffectOutcome;
        use crate::server::game_state::GameState;
        let mut pl = testing_all_characters::testing_pm();
        // push default effect
        pl.current_player
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect::default());
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
            .push(GameAtkEffect {
                processed_effect_param: build_cooldown_effect(),
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
            .push(GameAtkEffect {
                processed_effect_param: build_hot_effect_individual(),
                effect_outcome: EffectOutcome {
                    full_amount_tx: 30,
                    ..Default::default()
                },
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
            .push(GameAtkEffect {
                processed_effect_param: build_dot_effect_individual(),
                effect_outcome: EffectOutcome {
                    full_amount_tx: -20,
                    ..Default::default()
                },
                ..Default::default()
            });
        let (logs, hot_and_dot) = pl
            .current_player
            .character_rounds_info
            .process_hot_and_dot(gs.current_turn_nb);
        assert_eq!(2, logs.len()); // hot + dot
        assert_eq!(10, hot_and_dot); // 30(hot) - 20 (dot)
    }

    /// clear_scenario must reset speed to 0, restore stat maxima inflated by active
    /// ChangeMaxStat* effects (e.g. speed_regen, dodge), and set HP/Mana/Vigor to full.
    #[test]
    fn unit_clear_scenario_resets_stats() {
        use crate::character_mod::{
            buffers::{BufKinds, Buffer},
            effect::{EffectParam, ProcessedEffectParam},
        };
        use crate::common::constants::{all_target_const::TARGET_HIMSELF, reach_const::INDIVIDUAL};

        let mut pm = testing_pm();

        let old_dodge_max = pm.active_heroes[0].stats.all_stats[DODGE].max;
        let old_speed_regen_max = pm.active_heroes[0].stats.all_stats[SPEED_REGEN].max;
        let old_hp_max = pm.active_heroes[0].stats.all_stats[HP].max;
        let old_mana_max = pm.active_heroes[0].stats.all_stats[MANA].max;

        // Inject ChangeMaxStat +20 on Dodge (simulates an active mid-scenario buff).
        let dodge_ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::ChangeMaxStat,
                value: 20,
                is_percent: false,
                stats_name: DODGE.to_string(),
                ..Default::default()
            },
            nb_turns: 3,
            target_kind: TARGET_HIMSELF.to_string(),
            reach: INDIVIDUAL.to_string(),
            ..Default::default()
        };
        pm.active_heroes[0]
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: ProcessedEffectParam {
                    input_effect_param: dodge_ep,
                    number_of_applies: 1,
                    ..Default::default()
                },
                ..Default::default()
            });
        pm.active_heroes[0]
            .stats
            .set_stats_on_effect(DODGE, 20, false, true);

        // Inject ChangeMaxStat +5 on SPEED_REGEN (simulates a speed-regen buff).
        let sr_ep = EffectParam {
            buffer: Buffer {
                kind: BufKinds::ChangeMaxStat,
                value: 5,
                is_percent: false,
                stats_name: SPEED_REGEN.to_string(),
                ..Default::default()
            },
            nb_turns: 3,
            target_kind: TARGET_HIMSELF.to_string(),
            reach: INDIVIDUAL.to_string(),
            ..Default::default()
        };
        pm.active_heroes[0]
            .character_rounds_info
            .all_effects
            .push(GameAtkEffect {
                processed_effect_param: ProcessedEffectParam {
                    input_effect_param: sr_ep,
                    number_of_applies: 1,
                    ..Default::default()
                },
                ..Default::default()
            });
        pm.active_heroes[0]
            .stats
            .set_stats_on_effect(SPEED_REGEN, 5, false, true);

        // Confirm effects are live before the clear.
        assert_eq!(
            old_dodge_max + 20,
            pm.active_heroes[0].stats.all_stats[DODGE].max,
            "pre-check: Dodge should be buffed"
        );
        assert_eq!(
            old_speed_regen_max + 5,
            pm.active_heroes[0].stats.all_stats[SPEED_REGEN].max,
            "pre-check: Speed regen should be buffed"
        );

        // Simulate accumulated speed (e.g. after a supplementary-attack reset_speed call).
        pm.active_heroes[0].stats.get_mut_value(SPEED).current = 50;
        // Partially drain HP and Mana.
        pm.active_heroes[0].stats.get_mut_value(HP).current = 10;
        pm.active_heroes[0].stats.get_mut_value(MANA).current = 5;

        // --- Act ---
        pm.clear_scenario();

        // --- Assert ---
        let hero = &pm.active_heroes[0];

        assert_eq!(
            0, hero.stats.all_stats[SPEED].current,
            "Speed must be 0 after clear_scenario"
        );
        assert_eq!(
            old_dodge_max, hero.stats.all_stats[DODGE].max,
            "Dodge max must be restored after clear_scenario"
        );
        assert_eq!(
            old_speed_regen_max, hero.stats.all_stats[SPEED_REGEN].max,
            "Speed regen max must be restored after clear_scenario"
        );
        assert_eq!(
            old_hp_max, hero.stats.all_stats[HP].current,
            "HP must be restored to max after clear_scenario"
        );
        assert_eq!(
            old_mana_max, hero.stats.all_stats[MANA].current,
            "Mana must be restored to max after clear_scenario"
        );
        assert_eq!(
            0, hero.stats.all_stats[BERSERK].current,
            "Berserk must be 0 after clear_scenario"
        );
        assert_eq!(
            0, hero.stats.all_stats[AGGRO].current,
            "Aggro must be 0 after clear_scenario"
        );
    }

    /// clear_scenario must clamp any stat inflated above max by the OverHealBoostStat passive.
    #[test]
    fn unit_clear_scenario_resets_overheal_passive_stat_boost() {
        use crate::common::constants::stats_const::PHYSICAL_POWER;

        let mut pm = testing_all_characters::dxrpg_pm();
        // Use Azrak who carries the OverHealBoostStat passive on Physical power.
        let azrak_id = "Azrak_Ombresang_#1";
        let phys_pow_max = pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .max;

        // Simulate the passive having accumulated a large uncapped boost across a scenario.
        pm.get_mut_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .get_mut_value(PHYSICAL_POWER)
            .current = phys_pow_max + 200;

        pm.clear_scenario();

        let after = pm
            .get_active_hero_character(azrak_id)
            .unwrap()
            .stats
            .all_stats[PHYSICAL_POWER]
            .current;
        assert_eq!(
            after, phys_pow_max,
            "Physical power current must be clamped back to max after clear_scenario"
        );
    }
}
