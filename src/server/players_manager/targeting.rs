use crate::{
    character_mod::{
        attack_type::AttackType,
        buffers::BufKinds,
        character::{Character, CharacterKind},
        inventory::Consumable,
    },
    common::constants::{
        all_target_const::{TARGET_ALL_ALLIES, TARGET_ALLY, TARGET_ENNEMY, TARGET_HIMSELF},
        reach_const::{INDIVIDUAL, ZONE},
    },
};

use super::PlayerManager;

impl PlayerManager {
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

    /// Helper function to set targets for a given collection of characters
    /// Extracted to avoid code duplication between heroes and bosses targeting
    fn has_resurrect_effect(atk: &AttackType) -> bool {
        atk.all_effects
            .iter()
            .any(|e| e.buffer.kind == BufKinds::Resurrect)
    }

    fn set_targets_for_collection(
        characters: &mut [Character],
        launcher_id_name: &str,
        atk: &AttackType,
        is_ally_condition: bool,
        is_ennemy_condition: bool,
    ) {
        let can_target_dead = is_ally_condition && Self::has_resurrect_effect(atk);
        let mut has_at_least_one_target = false;
        characters
            .iter_mut()
            .filter(|c| {
                let alive_or_targetable = c.stats.is_dead() == Some(false)
                    || (can_target_dead && c.stats.is_dead() == Some(true));
                alive_or_targetable
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
        let can_target_dead = is_ally_condition && Self::has_resurrect_effect(atk);
        let mut has_at_least_one_target = false;
        let mut nb = 0;
        characters
            .iter()
            .filter(|c| {
                let alive_or_targetable = c.stats.is_dead() == Some(false)
                    || (can_target_dead && c.stats.is_dead() == Some(true));
                alive_or_targetable
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

    /// Mark potential/current targets for a consumable, mirroring set_targeted_characters for attacks.
    /// Target type is derived from the consumable's first effect target_kind.
    /// Resurrection consumables (BufKinds::Resurrect) only mark dead allies; all others mark alive.
    pub fn set_targeted_characters_for_consumable(
        &mut self,
        launcher_id_name: &str,
        consumable: &Consumable,
    ) {
        self.reset_targeted_character();
        self.reset_potential_targeted_character();

        let Some(first_effect) = consumable.effects.first() else {
            return;
        };
        let target_kind = &first_effect.target_kind;
        let can_target_dead = consumable
            .effects
            .iter()
            .any(|e| e.buffer.kind == BufKinds::Resurrect);

        if target_kind == TARGET_HIMSELF {
            if let Some(launcher) = self.get_mut_active_character(launcher_id_name) {
                launcher.character_rounds_info.is_current_target = true;
                launcher.character_rounds_info.is_potential_target = true;
            }
            return;
        }

        if target_kind == TARGET_ALLY {
            let mut has_first = false;
            for c in self.active_heroes.iter_mut() {
                let is_targetable = if can_target_dead {
                    c.stats.is_dead() == Some(true)
                } else {
                    c.stats.is_dead() == Some(false)
                };
                if is_targetable {
                    if !has_first {
                        c.character_rounds_info.is_current_target = true;
                        has_first = true;
                    }
                    c.character_rounds_info.is_potential_target = true;
                }
            }
            return;
        }

        if target_kind == TARGET_ENNEMY {
            let mut has_first = false;
            for c in self.active_bosses.iter_mut() {
                if c.stats.is_dead() == Some(false) {
                    if !has_first {
                        c.character_rounds_info.is_current_target = true;
                        has_first = true;
                    }
                    c.character_rounds_info.is_potential_target = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::constants::stats_const::*;
    use crate::testing::testing_all_characters::{self, testing_pm};

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
}
