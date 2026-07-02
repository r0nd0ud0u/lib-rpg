use anyhow::{Result, bail};

use crate::{character_mod::effect::EffectOutcome, server::game_state::GameState};

use super::PlayerManager;

impl PlayerManager {
    /// Use a consumable from the shared party bag for the hero identified by `hero_id_name`.
    /// The consumable is removed from the party pool and applied to the hero.
    /// Returns an error if the hero or consumable is not found.
    pub fn use_party_consumable(
        &mut self,
        hero_id_name: &str,
        potion_name: &str,
        game_state: &GameState,
    ) -> Result<()> {
        let idx = self
            .party_consumables
            .iter()
            .position(|c| c.name == potion_name)
            .ok_or_else(|| anyhow::anyhow!("Party consumable '{}' not found", potion_name))?;
        let consumable = self.party_consumables.remove(idx);
        let hero = self
            .get_mut_active_hero_character(hero_id_name)
            .ok_or_else(|| anyhow::anyhow!("Hero '{}' not found", hero_id_name))?;
        let launcher_stats = hero.stats.clone();
        hero.apply_consumable_effects(&consumable, game_state, &launcher_stats)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    /// Remove a personal consumable from current_player and apply it to the named target.
    /// launcher_stats come from current_player so power scaling is always off the launcher.
    pub fn use_consumable_on_target(
        &mut self,
        potion_name: &str,
        target_id_name: &str,
        game_state: &GameState,
    ) -> Result<Vec<EffectOutcome>> {
        if !self.current_player.inventory.contains_potion(potion_name) {
            bail!("no {} in inventory", potion_name);
        }
        let consumable = self
            .current_player
            .inventory
            .consumables
            .iter()
            .find(|c| c.name == potion_name)
            .cloned()
            .unwrap();
        let launcher_stats = self.current_player.stats.clone();
        let launcher_id = self.current_player.id_name.clone();

        let outcomes = if target_id_name == launcher_id {
            self.current_player.apply_consumable_effects(
                &consumable,
                game_state,
                &launcher_stats,
            )?
        } else if let Some(target) = self
            .active_heroes
            .iter_mut()
            .find(|c| c.id_name == target_id_name)
        {
            target.apply_consumable_effects(&consumable, game_state, &launcher_stats)?
        } else if let Some(target) = self
            .active_bosses
            .iter_mut()
            .find(|c| c.id_name == target_id_name)
        {
            target.apply_consumable_effects(&consumable, game_state, &launcher_stats)?
        } else {
            bail!("target {} not found", target_id_name);
        };

        self.current_player.inventory.remove_potion(potion_name);
        Ok(outcomes)
    }

    /// Remove a party consumable and apply it to the named target.
    pub fn use_party_consumable_on_target(
        &mut self,
        potion_name: &str,
        target_id_name: &str,
        game_state: &GameState,
    ) -> Result<Vec<EffectOutcome>> {
        let idx = self
            .party_consumables
            .iter()
            .position(|c| c.name == potion_name)
            .ok_or_else(|| anyhow::anyhow!("party consumable {} not found", potion_name))?;
        let consumable = self.party_consumables.remove(idx);
        let launcher_stats = self.current_player.stats.clone();
        let launcher_id = self.current_player.id_name.clone();

        // Self-targeting: apply to current_player so that modify_active_character() can sync
        // it back to active_heroes without overwriting the HP change.
        if target_id_name == launcher_id {
            let outcomes = self.current_player.apply_consumable_effects(
                &consumable,
                game_state,
                &launcher_stats,
            )?;
            return Ok(outcomes);
        }

        if let Some(target) = self
            .active_heroes
            .iter_mut()
            .find(|c| c.id_name == target_id_name)
        {
            let outcomes =
                target.apply_consumable_effects(&consumable, game_state, &launcher_stats)?;
            Ok(outcomes)
        } else {
            self.party_consumables.insert(idx, consumable);
            bail!("target {} not found", target_id_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::server::game_state::GameState;
    use crate::testing::testing_all_characters;

    #[test]
    fn unit_use_party_consumable_ok_and_err() {
        use crate::character_mod::inventory::Consumable;
        use crate::testing::testing_all_characters::testing_pm;
        let mut pl = testing_pm();

        // error: consumable not found
        assert!(
            pl.use_party_consumable("test_#1", "NoSuchPotion", &GameState::default())
                .is_err()
        );

        // success: valid hero + valid potion
        pl.party_consumables.push(Consumable {
            name: "TestPotion".to_string(),
            ..Default::default()
        });
        assert!(
            pl.use_party_consumable("test_#1", "TestPotion", &GameState::default())
                .is_ok()
        );
        // consumable removed after use
        assert!(pl.party_consumables.is_empty());

        // error: hero not found (consumable removed by the function before hero check)
        pl.party_consumables.push(Consumable {
            name: "TestPotion2".to_string(),
            ..Default::default()
        });
        assert!(
            pl.use_party_consumable("no_hero", "TestPotion2", &GameState::default())
                .is_err()
        );
    }

    #[test]
    fn unit_use_consumable_on_target_self() {
        use crate::common::constants::stats_const::HP;
        let mut pm = testing_all_characters::testing_pm();
        pm.current_player.stats.all_stats[HP].current = 10;
        // sync reduced HP back to active_heroes so is_dead check is consistent
        let id = pm.current_player.id_name.clone();
        pm.modify_active_character(&id);

        pm.current_player.inventory.add_small_potion();
        // also sync the new potion into active_heroes
        pm.modify_active_character(&id);

        let gs = GameState::default();
        let result = pm.use_consumable_on_target("potion", &id, &gs);
        assert!(
            result.is_ok(),
            "use_consumable_on_target(self) must succeed"
        );
        assert!(
            pm.current_player.stats.all_stats[HP].current > 10,
            "current_player HP must increase after self-potion"
        );
        assert!(
            pm.current_player.inventory.consumables.is_empty(),
            "potion must be removed from current_player"
        );

        // simulate what use_potion_handler does: sync current_player back to active_heroes
        pm.modify_active_character(&id);
        assert!(
            pm.active_heroes
                .iter()
                .find(|c| c.id_name == id)
                .map(|c| c.stats.all_stats[HP].current)
                .unwrap_or(0)
                > 10,
            "active_heroes HP must reflect the heal after modify_active_character"
        );
    }

    #[test]
    fn unit_use_party_consumable_on_target_self() {
        use crate::character_mod::effect::build_hp_effect;
        use crate::character_mod::inventory::Consumable;
        use crate::common::constants::stats_const::HP;

        let mut pm = testing_all_characters::testing_pm();
        pm.current_player.stats.all_stats[HP].current = 10;
        let id = pm.current_player.id_name.clone();
        pm.modify_active_character(&id);

        pm.party_consumables.push(Consumable {
            name: "PartyHealPotion".to_owned(),
            effects: vec![build_hp_effect(50, false)],
            ..Default::default()
        });

        let gs = GameState::default();
        let result = pm.use_party_consumable_on_target("PartyHealPotion", &id, &gs);
        assert!(
            result.is_ok(),
            "use_party_consumable_on_target(self) must succeed"
        );
        assert!(
            pm.party_consumables.is_empty(),
            "party consumable must be removed after use"
        );
        assert!(
            pm.current_player.stats.all_stats[HP].current > 10,
            "current_player HP must increase after party self-potion"
        );

        // simulate modify_active_character: HP change must survive the sync
        pm.modify_active_character(&id);
        assert!(
            pm.active_heroes
                .iter()
                .find(|c| c.id_name == id)
                .map(|c| c.stats.all_stats[HP].current)
                .unwrap_or(0)
                > 10,
            "active_heroes HP must reflect the heal after modify_active_character"
        );
    }
}
