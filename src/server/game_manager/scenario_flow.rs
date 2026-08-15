use crate::{
    character_mod::{
        character::Character,
        class::Class,
        equipment::Equipment,
        experience::{build_exp_to_next_level, build_experience},
        loot::LootType,
    },
    common::constants::character_const::ULTIMATE_LEVEL,
    server::{end_of_scenario::LevelUp, scenario::ScenarioState},
    shop::build_consumable_by_name,
};
use anyhow::Result;

use super::GameManager;

impl GameManager {
    /// Set active bosses from the current scenario's boss patterns.
    /// Bosses whose name matches a pattern in the current scenario are cloned and
    /// pushed into `pm.active_bosses` with a unique id_name (`"<name>_#<n>"`).
    pub fn set_active_bosses(&mut self, all_bosses: &[Character]) {
        self.current_scenario
            .boss_patterns
            .iter()
            .for_each(|(boss_name, _)| {
                if let Some(b) = all_bosses.iter().find(|b| b.db_full_name == *boss_name) {
                    let mut boss_to_push = b.clone();
                    boss_to_push.id_name = format!(
                        "{}_#{}",
                        boss_to_push.db_full_name,
                        1 + self
                            .pm
                            .get_nb_of_active_bosses_by_name(&boss_to_push.db_full_name)
                    );
                    self.pm.active_bosses.push(boss_to_push);
                } else {
                    tracing::warn!("Boss {} not found in data manager, skipping it", boss_name);
                }
            });
    }

    pub fn load_next_scenario(&mut self) -> Result<()> {
        // Debug-only: helps diagnose reports of the Scenarios tab showing stale progress
        // after a transition — trace when states_scenarios actually flips so it can be
        // compared against when the client-facing ServerEvent broadcast fires.
        tracing::debug!(
            "load_next_scenario: start, current={}",
            self.current_scenario.name
        );
        // update current scenario state
        if let Some((_, state)) = self
            .states_scenarios
            .iter_mut()
            .find(|(name, _)| *name == &self.current_scenario.name)
        {
            *state = ScenarioState::Completed;
        }
        let current_level = self.current_scenario.level;
        let current_universe = self.current_scenario.universe.clone();
        // get the next scenario with the next level in the same universe.
        // When current_universe is empty (default scenario at game start) skip the
        // universe filter so the first real scenario is found regardless of universe.
        let Some(scenario) = self
            .all_scenarios
            .iter()
            .find(|s| {
                s.level == current_level + 1
                    && (current_universe.is_empty() || s.universe == current_universe)
            })
            .cloned()
        else {
            return Err(anyhow::anyhow!(
                "No next scenario found for level {}",
                current_level + 1
            ));
        };
        // update scenario state in map
        if let Some((_, state)) = self
            .states_scenarios
            .iter_mut()
            .find(|(name, _)| *name == &scenario.name)
        {
            *state = ScenarioState::InProgress;
        }
        // update current scenario
        self.current_scenario = scenario;

        if self.current_scenario.level > 1 {
            // accumulate kills from the completed scenario before clearing
            let scenario_kills = self
                .pm
                .active_bosses
                .iter()
                .filter(|b| b.stats.is_dead().unwrap_or(false))
                .count();
            self.game_state.accumulated_kills += scenario_kills;
            // clear previous scenario
            self.game_state.clear_scenario();
            self.pm.clear_scenario();
            // set active bosses for the new scenario from the stored roster
            // do it before start new turn and after clearing a scenario
            let all_bosses = self.pm.all_bosses.clone();
            self.set_active_bosses(&all_bosses);
            let _ = self.start_new_turn();
        } else {
            // set active bosses for the new scenario from the stored roster
            let all_bosses = self.pm.all_bosses.clone();
            self.set_active_bosses(&all_bosses);
        }

        tracing::debug!(
            "load_next_scenario: done, new_current={}, states_scenarios={:?}",
            self.current_scenario.name,
            self.states_scenarios
        );
        Ok(())
    }

    pub fn all_scenarios_completed(&self) -> bool {
        self.states_scenarios
            .values()
            .all(|state| *state == ScenarioState::Completed)
    }

    /// Process end-of-scenario rewards for every hero:
    /// - Add loot items matching the hero's class (equipment checked against the equipment database,
    ///   consumables and currency added directly)
    /// - Add experience gained from all defeated bosses and level up (with stat update) as needed
    /// - Automatically use all consumables in inventory (potions restore HP)
    ///   Process end of scenario struct to be sent to the frontend with the rewards and the level up info
    pub fn process_end_of_scenario(&mut self) {
        // Debug-only: helps diagnose reports of duplicate loot after a scenario — every
        // call to this function grants a full set of rewards, so if it's ever invoked
        // twice for the same scenario completion (e.g. from two different code paths
        // both reacting to "all bosses dead"), this line will appear twice in the logs.
        tracing::debug!(
            "process_end_of_scenario: scenario={}, active_heroes={}",
            self.current_scenario.name,
            self.pm.active_heroes.len()
        );
        // Total exp: sum from all bosses
        let total_exp: u64 = self
            .pm
            .active_bosses
            .iter()
            .map(|boss| build_experience(&boss.rank, boss.level))
            .sum();

        let loots = self.current_scenario.loots.clone();
        let equipment_table_flat: Vec<Equipment> = self
            .pm
            .equipment_table
            .values()
            .flatten()
            .cloned()
            .collect();

        // prepare end of scenario
        self.end_of_scenario.scenario_level = self.current_scenario.level;
        self.end_of_scenario.characters_levelup.clear();
        self.pm.active_heroes.iter().for_each(|hero| {
            self.end_of_scenario.characters_levelup.push(LevelUp {
                character_id_name: hero.id_name.clone(),
                new_level: hero.level,
                old_level: hero.level,
            });
        });

        for i in 0..self.pm.active_heroes.len() {
            let hero_class = self.pm.active_heroes[i].class.clone();

            // Add loot according to class
            for loot in &loots {
                let class_matches =
                    loot.classes.contains(&hero_class) || loot.classes.contains(&Class::Standard);
                if !class_matches {
                    continue;
                }
                match &loot.kind {
                    LootType::Equipment => {
                        if let Some(equipment) = equipment_table_flat
                            .iter()
                            .find(|e| e.unique_name == loot.name)
                            .cloned()
                        {
                            tracing::debug!(
                                "process_end_of_scenario: granting equipment '{}' (category {:?}) to {}",
                                equipment.unique_name,
                                equipment.category,
                                self.pm.active_heroes[i].id_name
                            );
                            self.pm.active_heroes[i]
                                .inventory
                                .add_equipment(&equipment, false);
                        } else {
                            tracing::warn!(
                                "Equipment '{}' not found in equipment database",
                                loot.name
                            );
                        }
                    }
                    LootType::Consumable => {
                        // Consumables go to the shared party bag (handled below).
                    }
                    LootType::Currency => {
                        self.pm.active_heroes[i].inventory.money += loot.level as u64;
                    }
                    LootType::Material => {}
                }
            }

            // Add experience and level up if needed
            self.pm.active_heroes[i].character_rounds_info.exp += total_exp;
            while self.pm.active_heroes[i].character_rounds_info.exp
                >= self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level
                && self.pm.active_heroes[i].level < ULTIMATE_LEVEL
            {
                self.pm.active_heroes[i].character_rounds_info.exp -= self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level;
                self.pm.active_heroes[i].level += 1;
                self.pm.active_heroes[i].stats.update_stats_to_next_level();
                // Grant a talent skill point every level, plus a bonus every 5th level
                self.pm.active_heroes[i].talents.skill_points += 1;
                self.pm.active_heroes[i].talents.has_unseen_points = true;
                if self.pm.active_heroes[i].level.is_multiple_of(5) {
                    self.pm.active_heroes[i].talents.skill_points += 1;
                }
                // Recompute the threshold for the new level
                self.pm.active_heroes[i]
                    .character_rounds_info
                    .exp_to_next_level = build_exp_to_next_level(
                    &self.pm.active_heroes[i].rank,
                    &self.pm.active_heroes[i].class,
                    self.pm.active_heroes[i].level,
                );
                // update end of scenario
                if let Some(level_up) = self
                    .end_of_scenario
                    .characters_levelup
                    .iter_mut()
                    .find(|lu| lu.character_id_name == self.pm.active_heroes[i].id_name)
                {
                    level_up.new_level = self.pm.active_heroes[i].level;
                }
            }
        }

        // Add consumable loot to the shared party bag (once per loot item, not per hero).
        for loot in &loots {
            if loot.kind != LootType::Consumable {
                continue;
            }
            let any_hero_matches = self.pm.active_heroes.iter().any(|hero| {
                loot.classes.contains(&hero.class) || loot.classes.contains(&Class::Standard)
            });
            if any_hero_matches && let Some(consumable) = build_consumable_by_name(&loot.name) {
                self.pm.party_consumables.push(consumable);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::common::constants::stats_const::*;
    use crate::server::game_state::GameStatus;
    use crate::testing::testing_all_characters::{
        self, testing_game_manager, testing_test_ally1_vs_test_boss1,
    };
    #[test]
    fn unit_load_next_scenario() {
        use crate::server::scenario::ScenarioState;

        let mut gm = testing_all_characters::dxrpg_game_manager();

        // dxrpg loads lotr scenarios; states start as NotStarted
        let stage1_name = "Patrouille Gobeline".to_owned();
        let stage2_name = "Embuscade Gobeline".to_owned();
        assert_eq!(gm.states_scenarios[&stage1_name], ScenarioState::NotStarted);
        assert_eq!(gm.states_scenarios[&stage2_name], ScenarioState::NotStarted);

        // set stage 1 as current (simulates game start on stage 1)
        let stage1 = gm
            .all_scenarios
            .iter()
            .find(|s| s.name == stage1_name)
            .cloned()
            .unwrap();
        gm.current_scenario = stage1;
        gm.states_scenarios
            .insert(stage1_name.clone(), ScenarioState::InProgress);

        // damage heroes and drain their energy to verify it carries over unchanged into the next scenario
        for hero in gm.pm.active_heroes.iter_mut() {
            hero.stats.get_mut_value(HP).current = 1;
            hero.stats.get_mut_value(MANA).current = 0;
            hero.stats.get_mut_value(VIGOR).current = 0;
            hero.stats.get_mut_value(BERSERK).current = 0;
        }

        // load stage 2
        let result = gm.load_next_scenario();
        assert!(result.is_ok(), "loading stage 2 should succeed");

        // stage 1 must be Completed
        assert_eq!(
            gm.states_scenarios[&stage1_name],
            ScenarioState::Completed,
            "stage 1 should be Completed after loading stage 2"
        );
        // stage 2 must be InProgress
        assert_eq!(
            gm.states_scenarios[&stage2_name],
            ScenarioState::InProgress,
            "stage 2 should be InProgress after being loaded"
        );
        // current_scenario must be stage 2
        assert_eq!(gm.current_scenario.name, stage2_name);

        // active_bosses count must equal the stage 2 boss patterns
        assert_eq!(
            gm.pm.active_bosses.len(),
            gm.current_scenario.boss_patterns.len(),
            "active_bosses should match stage 2 boss patterns count"
        );

        // heroes must have effects cleared, but HP/Mana/Vigor carry over unchanged
        for hero in gm.pm.active_heroes.iter() {
            assert_eq!(
                hero.stats.all_stats[HP].current, 1,
                "hero {} HP should NOT be restored on scenario transition",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[MANA].current, 0,
                "hero {} Mana should NOT be restored on scenario transition",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[VIGOR].current, 0,
                "hero {} Vigor should NOT be restored on scenario transition",
                hero.db_full_name
            );
            assert_eq!(
                hero.stats.all_stats[BERSERK].current, 0,
                "hero {} Berserk should NOT be restored on scenario load",
                hero.db_full_name
            );
            assert!(
                hero.character_rounds_info.all_effects.is_empty(),
                "hero {} should have no active effects after scenario transition",
                hero.db_full_name
            );
        }

        // all_scenarios_completed returns false (stage 2 still in progress)
        assert!(!gm.all_scenarios_completed());
    }

    #[test]
    fn unit_set_active_bosses() {
        use crate::testing::testing_all_characters::dxrpg_dm;

        let dm = dxrpg_dm();
        let mut gm = testing_all_characters::dxrpg_game_manager();

        // set stage 1 as current scenario so boss_patterns are in scope
        let stage1 = gm
            .all_scenarios
            .iter()
            .find(|s| s.level == 1)
            .cloned()
            .unwrap();
        gm.current_scenario = stage1;

        // no bosses yet
        gm.pm.active_bosses.clear();
        assert_eq!(gm.pm.active_bosses.len(), 0);

        gm.set_active_bosses(&dm.all_bosses);

        // the number of active bosses must match the number of boss_patterns entries
        // that have a matching entry in dm.all_bosses
        let expected = gm
            .current_scenario
            .boss_patterns
            .keys()
            .filter(|name| dm.all_bosses.iter().any(|b| &b.db_full_name == *name))
            .count();
        assert_eq!(
            gm.pm.active_bosses.len(),
            expected,
            "active_bosses count should match boss_patterns with a known boss"
        );

        // each active boss must have the correct id_name suffix format
        for boss in &gm.pm.active_bosses {
            assert!(
                boss.id_name.contains("_#"),
                "id_name '{}' should contain '_#'",
                boss.id_name
            );
        }
    }

    // -------------------------------------------------------------------------
    // process_end_of_scenario tests
    // -------------------------------------------------------------------------

    #[test]
    fn unit_end_of_scenario_equipment_loot_matching_class() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Both test heroes are Standard class.
        // Create a scenario with one equipment loot targeting Standard heroes.
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "starting right weapon".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        gm.process_end_of_scenario();

        // Both heroes must now have the equipment in their inventory
        for hero in &gm.pm.active_heroes {
            let has_it = hero
                .inventory
                .equipments
                .values()
                .flatten()
                .any(|e| e.unique_name == "starting right weapon");
            assert!(
                has_it,
                "hero '{}' should have received the equipment",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_equipment_loot_no_class_match() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Both test heroes are Standard.
        // Equipment loot only for Warrior → heroes must NOT receive an extra copy.
        let belts_before: Vec<usize> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| {
                h.inventory
                    .equipments
                    .values()
                    .flatten()
                    .filter(|e| e.unique_name == "starting belt")
                    .count()
            })
            .collect();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "starting belt".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                classes: vec![Class::Warrior],
            }],
            universe: String::new(),
        };

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            let belts_after = hero
                .inventory
                .equipments
                .values()
                .flatten()
                .filter(|e| e.unique_name == "starting belt")
                .count();
            assert_eq!(
                belts_after, belts_before[idx],
                "hero '{}' belt count should not change (class mismatch)",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_equipment_loot_unknown_equipment() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Record initial equipment count per hero
        let equip_before: Vec<usize> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.inventory.equipments.values().map(|v| v.len()).sum())
            .collect();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "non_existent_equipment".to_string(),
                kind: LootType::Equipment,
                rank: Rank::Common,
                level: 1,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        // Must not panic; unknown equipment is just warned about and skipped
        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            let total_equip: usize = hero.inventory.equipments.values().map(|v| v.len()).sum();
            assert_eq!(
                total_equip, equip_before[idx],
                "hero '{}' equipment count should not change for unknown loot name",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_consumable_loot() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "potion".to_string(),
                kind: LootType::Consumable,
                rank: Rank::Common,
                level: 1,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        gm.process_end_of_scenario();

        // Consumables must go to the shared party bag — not to individual heroes.
        let in_party_bag = gm.pm.party_consumables.iter().any(|c| c.name == "potion");
        assert!(
            in_party_bag,
            "consumable should land in the party bag, not in individual inventories"
        );

        // Personal inventories must be untouched.
        for hero in &gm.pm.active_heroes {
            let in_personal_bag = hero
                .inventory
                .consumables
                .iter()
                .any(|c| c.name == "potion");
            assert!(
                !in_personal_bag,
                "hero '{}' should NOT have the consumable in their personal bag",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_loot_consumables_use_shop_definitions() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();

        for (potion_name, rank) in [
            ("potion of resurrection", Rank::Advanced),
            ("mana potion", Rank::Intermediate),
            ("vigor potion", Rank::Common),
            ("berserk potion", Rank::Advanced),
        ] {
            let original_len = gm.pm.party_consumables.len();
            gm.current_scenario = Scenario {
                name: "test".to_string(),
                description: "test".to_string(),
                boss_patterns: HashMap::new(),
                level: 1,
                loots: vec![Loot {
                    name: potion_name.to_string(),
                    kind: LootType::Consumable,
                    rank,
                    level: 1,
                    classes: vec![Class::Standard],
                }],
                universe: String::new(),
            };
            gm.process_end_of_scenario();
            let found = gm
                .pm
                .party_consumables
                .iter()
                .skip(original_len)
                .any(|c| c.name == potion_name);
            assert!(
                found,
                "'{potion_name}' should be in party bag after end_of_scenario"
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_currency_loot() {
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "gold".to_string(),
                kind: LootType::Currency,
                rank: Rank::Common,
                level: 100,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        // Test heroes already have money: 100 in their JSON
        let money_before: Vec<u64> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.inventory.money)
            .collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.inventory.money,
                money_before[idx] + 100,
                "hero '{}' should have received 100 gold",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_exp_and_level_up() {
        // Test setup: 2 bosses, each rank Common level 1 → 100 exp each → 200 total
        //
        // "test" hero:  exp=50, exp_to_next_level(Common, Standard, 1)=100
        //   50 + 200 = 250 → level-up to 2 (exp=150), new threshold=200 → 150 < 200 → stop at level 2
        //
        // "test2" hero: exp=0,  exp_to_next_level(Common, Standard, 1)=100
        //   0 + 200 = 200 → level-up to 2 (exp=100), new threshold=200 → 100 < 200 → stop at level 2
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            loots: vec![],
            level: 1,
            universe: String::new(),
        };

        let old_hp_max: Vec<u64> = gm
            .pm
            .active_heroes
            .iter()
            .map(|h| h.stats.all_stats[HP].max)
            .collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.level, 2,
                "hero '{}' should be level 2 after 200 exp (dynamic threshold grows to 200 at level 2)",
                hero.id_name
            );
            // exp_to_next_level must now reflect the new level
            assert_eq!(
                hero.character_rounds_info.exp_to_next_level,
                200, // build_exp_to_next_level(Common, Standard, 2) = 200
                "hero '{}' exp_to_next_level should be 200 at level 2",
                hero.id_name
            );
            // Stats must have been updated upward on level-up
            assert!(
                hero.stats.all_stats[HP].max > old_hp_max[idx],
                "hero '{}' HP max should have increased after leveling up",
                hero.id_name
            );
            // A single level-up (1 -> 2, not a multiple of 5) grants exactly 1 skill point
            assert_eq!(
                hero.talents.skill_points, 1,
                "hero '{}' should have earned 1 skill point for reaching level 2",
                hero.id_name
            );
        }
        // assess end of scenario LevelUp
        assert_eq!(gm.end_of_scenario.characters_levelup.len(), 2); // 2 heroes
        gm.end_of_scenario.characters_levelup.iter().for_each(|lu| {
            assert_eq!(
                lu.new_level, 2,
                "LevelUp record should show new level 2 for hero '{}'",
                lu.character_id_name
            );
            assert_eq!(
                lu.old_level, 1,
                "LevelUp record should show old level 1 for hero '{}'",
                lu.character_id_name
            );
        });
    }

    #[test]
    fn unit_talent_skill_points_milestone_bonus_at_level_5() {
        // Each call awards 200 exp (2 Common lvl1 bosses). Thresholds by level (Common,
        // Standard): 1->2:100, 2->3:200, 3->4:300, 4->5:400. Reaching level 5 from level 1
        // takes 4 level-ups (2,3,4,5), one of which (5) is a multiple-of-5 milestone:
        // total skill points = 4 + 1 bonus = 5.
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            loots: vec![],
            level: 1,
            universe: String::new(),
        };

        let mut iterations = 0;
        while gm.pm.active_heroes.iter().any(|h| h.level < 5) && iterations < 20 {
            gm.process_end_of_scenario();
            iterations += 1;
        }

        for hero in &gm.pm.active_heroes {
            assert_eq!(
                hero.level, 5,
                "hero '{}' should have reached level 5",
                hero.id_name
            );
            assert_eq!(
                hero.talents.skill_points, 5,
                "hero '{}' should have earned 5 skill points reaching level 5 (4 level-ups + 1 milestone bonus)",
                hero.id_name
            );
            assert!(
                hero.talents.has_unseen_points,
                "hero '{}' should have the talent notification badge lit after earning points",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_no_level_up_when_exp_insufficient() {
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let mut gm = testing_game_manager();
        // Remove all bosses so total_exp = 0
        gm.pm.active_bosses.clear();
        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            loots: vec![],
            level: 1,
            universe: String::new(),
        };

        let levels_before: Vec<u64> = gm.pm.active_heroes.iter().map(|h| h.level).collect();

        gm.process_end_of_scenario();

        for (idx, hero) in gm.pm.active_heroes.iter().enumerate() {
            assert_eq!(
                hero.level, levels_before[idx],
                "hero '{}' should not have leveled up with 0 exp",
                hero.id_name
            );
        }
    }

    #[test]
    fn unit_end_of_scenario_triggered_via_game_flow() {
        // Verify that eval_end_of_round sets EndOfScenario and processes rewards
        // when all bosses are killed in a single hit.
        use crate::character_mod::class::Class;
        use crate::character_mod::loot::{Loot, LootType};
        use crate::character_mod::rank::Rank;
        use crate::server::scenario::Scenario;
        use std::collections::HashMap;

        let (mut gm, _, _) = testing_test_ally1_vs_test_boss1();

        gm.current_scenario = Scenario {
            name: "test".to_string(),
            description: "test".to_string(),
            boss_patterns: HashMap::new(),
            level: 1,
            loots: vec![Loot {
                name: "gold".to_string(),
                kind: LootType::Currency,
                rank: Rank::Common,
                level: 50,
                classes: vec![Class::Standard],
            }],
            universe: String::new(),
        };

        // Kill all bosses
        for boss in gm.pm.active_bosses.iter_mut() {
            boss.stats.all_stats.get_mut(HP).unwrap().current = 0;
        }

        // Set target and launch — eval_end_of_round sees all bosses dead
        gm.pm
            .get_mut_active_boss_character("test_boss1_#1")
            .unwrap()
            .character_rounds_info
            .is_current_target = true;
        gm.launch_attack(None);

        assert_eq!(
            gm.game_state.status,
            GameStatus::EndOfScenario,
            "status should be EndOfScenario"
        );
        // Rewards must have been processed: each Standard hero got 50 gold on top of their starting 100
        for hero in &gm.pm.active_heroes {
            assert!(
                hero.inventory.money >= 50,
                "hero '{}' should have received 50 gold after end-of-scenario",
                hero.id_name
            );
        }
    }
}
