use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::character_mod::{attack_type::AtksInfo, effect::EffectOutcome};

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StatsInGame {
    pub all_atk_info: Vec<AtksInfo>,
}

impl StatsInGame {
    pub fn update_by_effectoutcome(&mut self, eo: &EffectOutcome) {
        // Try to find existing attack
        if let Some(atk_info) = self
            .all_atk_info
            .iter_mut()
            .find(|item| item.atk_name == eo.atk)
        {
            // Increment usage counter
            atk_info.nb_use += 1;

            // Update damage for this target (insert if missing)
            *atk_info
                .all_damages_by_target
                .entry(eo.target_kind.clone())
                .or_default() += eo.real_hp_amount_tx;
        } else {
            // First time this attack appears
            let mut damages = IndexMap::new();
            damages.insert(eo.target_kind.clone(), eo.real_hp_amount_tx);

            self.all_atk_info.push(AtksInfo {
                atk_name: eo.atk.clone(),
                nb_use: 1,
                all_damages_by_target: damages,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unit_update_by_effectoutcome() {
        let mut stats = StatsInGame::default();
        let eo = EffectOutcome {
            atk: "Fireball".to_string(),
            target_kind: "Goblin".to_string(),
            real_hp_amount_tx: -30,
            ..Default::default()
        };
        stats.update_by_effectoutcome(&eo);
        assert_eq!(stats.all_atk_info.len(), 1);
        assert_eq!(stats.all_atk_info[0].atk_name, "Fireball");
        assert_eq!(stats.all_atk_info[0].nb_use, 1);
        assert_eq!(
            stats.all_atk_info[0].all_damages_by_target.get("Goblin"),
            Some(&-30)
        );
        stats.update_by_effectoutcome(&eo);
        assert_eq!(stats.all_atk_info.len(), 1);
        assert_eq!(stats.all_atk_info[0].atk_name, "Fireball");
        assert_eq!(stats.all_atk_info[0].nb_use, 2);
        assert_eq!(
            stats.all_atk_info[0].all_damages_by_target.get("Goblin"),
            Some(&-60)
        );
    }
}
