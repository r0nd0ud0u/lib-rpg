use anyhow::Result;

use crate::{
    character_mod::{attack_type::AttackType, buffers::BufKinds, character::CharacterKind},
    common::{
        constants::{character_const::*, stats_const::*},
        log_data::{LogData, const_colors::LIGHT_GREEN},
    },
    server::game_state::GameState,
};

use super::PlayerManager;

impl PlayerManager {
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
                logs = self.current_player.new_round(
                    game_state.current_turn_nb,
                    self.process_launchable_atks(game_state.current_turn_nb),
                );

                // update the active character
                self.modify_active_character(id_name);

                Ok(logs)
            }
            None => {
                anyhow::bail!("Character '{}' not found", id_name)
            }
        }
    }

    /// Fires the `IsDamageTxHealNeedyAlly` passive for `launcher_id_name` if enabled:
    /// heals the most-needy alive hero by `pct`% of `damage_tx`.
    /// Called immediately after an attack deals damage so the heal appears in the same turn's log.
    pub(crate) fn apply_damage_tx_heal_passive(
        &mut self,
        launcher_id_name: &str,
        damage_tx: i64,
    ) -> Vec<LogData> {
        let mut logs = Vec::new();

        if damage_tx <= 0 {
            return logs;
        }

        let pct = {
            let Some(launcher) = self.get_active_hero_character(launcher_id_name) else {
                return logs;
            };
            let Some(buf) = launcher
                .character_rounds_info
                .get_buffer_by_type(&BufKinds::IsDamageTxHealNeedyAlly)
            else {
                return logs;
            };
            if !buf.is_passive || !buf.is_passive_enabled || buf.value <= 0 {
                return logs;
            }
            buf.value
        };

        let heal_amount = (damage_tx * pct / 100) as u64;
        if heal_amount == 0 {
            return logs;
        }

        // Find the alive hero with the lowest HP ratio (current * 10000 / max).
        let target_id = self
            .active_heroes
            .iter()
            .filter(|c| c.stats.is_dead() == Some(false))
            .min_by_key(|c| {
                c.stats
                    .all_stats
                    .get(HP)
                    .filter(|s| s.max > 0)
                    .map(|s| s.current * 10000 / s.max)
                    .unwrap_or(u64::MAX)
            })
            .map(|c| c.id_name.clone());

        let Some(target_id) = target_id else {
            return logs;
        };

        let Some(target) = self
            .active_heroes
            .iter_mut()
            .find(|c| c.id_name == target_id)
        else {
            return logs;
        };
        let short_name = target.short_name.clone();
        let Some(hp) = target.stats.all_stats.get_mut(HP) else {
            return logs;
        };
        let new_hp = (hp.current + heal_amount).min(hp.max);
        let real_heal = new_hp - hp.current;
        hp.current = new_hp;
        if real_heal > 0 {
            logs.push(LogData {
                message: format!(
                    "\u{26a1} Passive({}): {} \u{2190} +{} HP ({}% of {} damage TX)",
                    launcher_id_name, short_name, real_heal, pct, damage_tx
                ),
                color: LIGHT_GREEN.to_string(),
            });
        }

        logs
    }

    /// Process the start of a new turn by incrementing counter effects, resetting first round booleans and applying regen stats.
    pub fn start_new_turn(&mut self, is_first_turn: bool) {
        // Increment turn effects
        self.increment_counter_effect();
        // Reset new round boolean for characters
        self.reset_is_first_round();
        // Apply regen stats but not in first turn
        if !is_first_turn {
            self.apply_regen_stats(CharacterKind::Boss);
            self.apply_regen_stats(CharacterKind::Hero);
        }
    }

    pub fn process_sup_atk_turn(&mut self, launcher_type: CharacterKind) -> Vec<String> {
        let player_list = if launcher_type == CharacterKind::Hero {
            &mut self.active_heroes
        } else {
            &mut self.active_bosses
        };
        for pl in player_list {
            if pl.stats.is_dead().unwrap_or(false) {
                continue;
            }
            let speed = pl
                .stats
                .all_stats
                .get(SPEED)
                .map(|s| s.current)
                .unwrap_or(0);
            if speed >= SPEED_THRESHOLD {
                pl.stats.reset_speed();
                return vec![pl.id_name.clone()];
            }
        }
        vec![]
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

    /// Process the boss target at the start of the turn by setting the hero with the highest aggro as current target.
    /// If all heroes are dead, no target is set.
    /// If the current player is a hero, no target is set.
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

    pub fn process_launchable_atks(&self, current_turn_nb: usize) -> Vec<AttackType> {
        // assess potential target
        let mut launchable_attacks = Vec::new();

        for atk in self.current_player.attacks_list.values() {
            let can_be_launched = self.current_player.can_be_launched(atk, current_turn_nb);
            let whatif_nb =
                self.whatif_set_targeted_characters(&self.current_player.id_name, &atk.name);
            if can_be_launched && whatif_nb > 0 {
                launchable_attacks.push(atk.clone());
            }
        }
        launchable_attacks
    }

    /// Check if the game is over by checking if all heroes or all bosses are dead.
    /// Returns a tuple of booleans (all_heroes_dead, all_bosses_dead).
    pub fn check_end_of_game(&self) -> (bool, bool) {
        let all_heroes_dead = self
            .active_heroes
            .iter()
            .all(|c| c.stats.is_dead() == Some(true));
        let all_bosses_dead = self
            .active_bosses
            .iter()
            .all(|c| c.stats.is_dead() == Some(true));
        (all_heroes_dead, all_bosses_dead)
    }
}

#[cfg(test)]
mod tests {
    use crate::server::game_state::GameState;
    use crate::testing::testing_all_characters;

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
    fn unit_process_launchable_atks() {
        let mut pl = testing_all_characters::testing_pm();
        // no problem of level
        pl.current_player.level = 100;
        // no problem of is_heal_atk_blocked
        pl.current_player.character_rounds_info.is_heal_atk_blocked = false;
        let launchable_atks = pl.process_launchable_atks(0);
        assert_eq!(pl.current_player.attacks_list.len(), launchable_atks.len()); // in the list, one is berserk atk type and test.json has not the berserk energy!!

        // case level under
        pl.current_player.level = 1;
        let launchable_atks = pl.process_launchable_atks(0);
        assert_eq!(13, launchable_atks.len()); // 13 on 17 are level 1

        // case is_heal_atk_blocked
        pl.current_player.character_rounds_info.is_heal_atk_blocked = true;
        pl.current_player.level = 100;
        let launchable_atks = pl.process_launchable_atks(0);
        assert_eq!(10, launchable_atks.len()); // 6 attacks are HP and linked to is_heal_atk_blocked condition
    }

    #[test]
    fn unit_passive_damage_tx_heal_needy_ally_fires() {
        use crate::character_mod::buffers::Buffer;
        use crate::common::constants::stats_const::HP;
        let mut pm = testing_all_characters::testing_pm();
        let launcher_id = pm.active_heroes[0].id_name.clone();

        // Install IsDamageTxHealNeedyAlly passive on hero[0] at 25%
        pm.active_heroes[0]
            .character_rounds_info
            .all_buffers
            .push(Buffer {
                kind: crate::character_mod::buffers::BufKinds::IsDamageTxHealNeedyAlly,
                value: 25,
                is_passive: true,
                is_passive_enabled: true,
                ..Default::default()
            });

        // Set launcher (hero[0]) to full HP so hero[1] (at HP=1) is the most needy
        let max_hp_launcher = pm.active_heroes[0].stats.all_stats[HP].max;
        pm.active_heroes[0]
            .stats
            .all_stats
            .get_mut(HP)
            .unwrap()
            .current = max_hp_launcher;

        // hero[1] starts at HP=1 (from test JSON) — lowest ratio → most needy
        let hero1_hp_before = pm.active_heroes[1].stats.all_stats[HP].current; // 1

        // Passive fires immediately with damage_tx=200 (simulates an attack dealing 200 damage)
        pm.apply_damage_tx_heal_passive(&launcher_id, 200);

        let hero1_hp_after = pm.active_heroes[1].stats.all_stats[HP].current;
        // 200 * 25 / 100 = 50
        assert_eq!(
            hero1_hp_after,
            hero1_hp_before + 50,
            "most needy ally must receive 25% of 200 damage TX"
        );
    }

    #[test]
    fn unit_passive_damage_tx_heal_needy_ally_noop_when_no_damage() {
        use crate::character_mod::buffers::Buffer;
        use crate::common::constants::stats_const::HP;
        let mut pm = testing_all_characters::testing_pm();
        let launcher_id = pm.active_heroes[0].id_name.clone();

        pm.active_heroes[0]
            .character_rounds_info
            .all_buffers
            .push(Buffer {
                kind: crate::character_mod::buffers::BufKinds::IsDamageTxHealNeedyAlly,
                value: 25,
                is_passive: true,
                is_passive_enabled: true,
                ..Default::default()
            });

        let hero1_hp_before = pm.active_heroes[1].stats.all_stats[HP].current;

        // damage_tx=0 → passive must be a no-op
        pm.apply_damage_tx_heal_passive(&launcher_id, 0);

        assert_eq!(
            pm.active_heroes[1].stats.all_stats[HP].current, hero1_hp_before,
            "zero damage TX means no passive heal"
        );
    }

    #[test]
    fn unit_passive_damage_tx_heal_needy_ally_disabled() {
        use crate::character_mod::buffers::Buffer;
        use crate::common::constants::stats_const::HP;
        let mut pm = testing_all_characters::testing_pm();
        let launcher_id = pm.active_heroes[0].id_name.clone();

        // passive exists but is disabled
        pm.active_heroes[0]
            .character_rounds_info
            .all_buffers
            .push(Buffer {
                kind: crate::character_mod::buffers::BufKinds::IsDamageTxHealNeedyAlly,
                value: 25,
                is_passive: true,
                is_passive_enabled: false, // disabled
                ..Default::default()
            });

        let hero1_hp_before = pm.active_heroes[1].stats.all_stats[HP].current;

        pm.apply_damage_tx_heal_passive(&launcher_id, 200);

        assert_eq!(
            pm.active_heroes[1].stats.all_stats[HP].current, hero1_hp_before,
            "disabled passive must not heal"
        );
    }

    #[test]
    fn unit_passive_damage_tx_heal_needy_ally_capped_at_max_hp() {
        use crate::character_mod::buffers::Buffer;
        use crate::common::constants::stats_const::HP;
        let mut pm = testing_all_characters::testing_pm();
        let launcher_id = pm.active_heroes[0].id_name.clone();

        pm.active_heroes[0]
            .character_rounds_info
            .all_buffers
            .push(Buffer {
                kind: crate::character_mod::buffers::BufKinds::IsDamageTxHealNeedyAlly,
                value: 25,
                is_passive: true,
                is_passive_enabled: true,
                ..Default::default()
            });

        // Set launcher (hero[0]) to full HP so hero[1] (at HP=1) is the most needy
        let max_hp_launcher = pm.active_heroes[0].stats.all_stats[HP].max;
        pm.active_heroes[0]
            .stats
            .all_stats
            .get_mut(HP)
            .unwrap()
            .current = max_hp_launcher;

        // hero[1] starts at HP=1 (most needy); track its max for the cap assertion
        let hp_max = pm.active_heroes[1].stats.all_stats[HP].max; // 135

        // Huge damage TX so heal would overflow HP max
        pm.apply_damage_tx_heal_passive(&launcher_id, 10_000);

        assert_eq!(
            pm.active_heroes[1].stats.all_stats[HP].current, hp_max,
            "heal must not exceed HP max"
        );
    }
}
