use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::{character_mod::attack_type::AtksInfo, server::players_manager::GameAtkEffect};

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StatsInGame {
    pub all_atk_info: Vec<AtksInfo>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, EnumIter)]
pub enum StatsInfoKind {
    #[default]
    Atk,
    Others,
}

impl fmt::Display for StatsInfoKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StatsInfoKind::Atk => "Atk",
            StatsInfoKind::Others => "Others",
        };
        write!(f, "{}", s)
    }
}

impl StatsInGame {
    pub fn update_by_game_atk_effect(&mut self, gae: &GameAtkEffect) {
        // Try to find existing attack
        if let Some(atk_info) = self
            .all_atk_info
            .iter_mut()
            .find(|item| item.atk_name == gae.atk_type.name)
        {
            // Increment usage counter
            atk_info.nb_use += 1;

            // Update damage for this target (insert if missing)
            *atk_info
                .all_damages_by_target
                .entry(gae.effect_outcome.target_id_name.clone())
                .or_default() += gae.effect_outcome.real_hp_amount_tx;
        } else {
            // First time this attack appears
            let mut damages = IndexMap::new();
            damages.insert(
                gae.effect_outcome.target_id_name.clone(),
                gae.effect_outcome.real_hp_amount_tx,
            );

            self.all_atk_info.push(AtksInfo {
                atk_name: gae.atk_type.name.clone(),
                nb_use: 1,
                all_damages_by_target: damages,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::character_mod::{attack_type::AttackType, effect::EffectOutcome};

    use super::*;
    #[test]
    fn unit_update_by_game_atk_effect() {
        let mut stats = StatsInGame::default();
        let gae = GameAtkEffect {
            atk_type: AttackType {
                name: "Fireball".to_owned(),
                ..Default::default()
            },
            effect_outcome: EffectOutcome {
                target_id_name: "Goblin".to_owned(),
                real_hp_amount_tx: -30,
                ..Default::default()
            },
            ..Default::default()
        };
        stats.update_by_game_atk_effect(&gae);
        assert_eq!(stats.all_atk_info.len(), 1);
        assert_eq!(stats.all_atk_info[0].atk_name, "Fireball");
        assert_eq!(stats.all_atk_info[0].nb_use, 1);
        assert_eq!(
            stats.all_atk_info[0].all_damages_by_target.get("Goblin"),
            Some(&-30)
        );
        stats.update_by_game_atk_effect(&gae);
        assert_eq!(stats.all_atk_info.len(), 1);
        assert_eq!(stats.all_atk_info[0].atk_name, "Fireball");
        assert_eq!(stats.all_atk_info[0].nb_use, 2);
        assert_eq!(
            stats.all_atk_info[0].all_damages_by_target.get("Goblin"),
            Some(&-60)
        );
    }
}
