use std::fmt;

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
    AtksCount,
    AtksAmount,
}

impl fmt::Display for StatsInfoKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StatsInfoKind::AtksCount => "Count",
            StatsInfoKind::AtksAmount => "Amount",
        };
        write!(f, "{}", s)
    }
}

impl StatsInGame {
    pub fn process_all_game_stats(&mut self, new_gaes: &Vec<GameAtkEffect>, atk_name: &str) {
        if let Some(atk_info) = self
            .all_atk_info
            .iter_mut()
            .find(|item| item.atk_name == atk_name)
        {
            // Increment usage counter
            atk_info.nb_use += 1;
        } else {
            self.all_atk_info.push(AtksInfo {
                atk_name: atk_name.to_owned(),
                nb_use: 1,
                ..Default::default()
            });
        }

        // get the atack info to update the damage
        if let Some(atk_info) = self
            .all_atk_info
            .iter_mut()
            .find(|item| item.atk_name == atk_name)
        {
            for gae in new_gaes {
                // Update damage for this target (insert if missing)
                if gae.effect_outcome.full_amount_tx > 0 {
                    let entry = atk_info
                        .totals_by_target
                        .entry(gae.effect_outcome.target_id_name.clone())
                        .or_default();
                    entry.total_full_heal += gae.effect_outcome.full_amount_tx;
                    entry.total_real_heal += gae.effect_outcome.real_amount_tx;
                }
                if gae.effect_outcome.full_amount_tx < 0 {
                    let entry = atk_info
                        .totals_by_target
                        .entry(gae.effect_outcome.target_id_name.clone())
                        .or_default();
                    entry.total_full_dmg += gae.effect_outcome.full_amount_tx;
                    entry.total_real_dmg += gae.effect_outcome.real_amount_tx;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::character_mod::{
        attack_type::{AccumulatedAtkInfo, AttackType},
        effect::EffectOutcome,
    };

    use super::*;
    #[test]
    fn unit_process_all_game_stats() {
        let mut stats_in_game = StatsInGame::default();
        let attack_type = AttackType {
            name: "Test Attack".to_string(),
            ..Default::default()
        };
        let game_atk_effect = GameAtkEffect {
            effect_outcome: EffectOutcome {
                full_amount_tx: 100,
                real_amount_tx: 50,
                target_id_name: "Target1".to_string(),
                is_critical: false,
                aggro_generated: 10,
            },
            ..Default::default()
        };
        stats_in_game.process_all_game_stats(&vec![game_atk_effect.clone()], &attack_type.name);

        assert_eq!(stats_in_game.all_atk_info.len(), 1);
        assert_eq!(stats_in_game.all_atk_info[0].atk_name, "Test Attack");
        assert_eq!(stats_in_game.all_atk_info[0].nb_use, 1);
        assert_eq!(
            stats_in_game.all_atk_info[0]
                .totals_by_target
                .get("Target1"),
            Some(&AccumulatedAtkInfo {
                total_full_heal: 100,
                total_full_dmg: 0,
                total_real_dmg: 0,
                total_real_heal: 50,
            })
        );

        stats_in_game.process_all_game_stats(&vec![game_atk_effect.clone()], &attack_type.name);
        assert_eq!(stats_in_game.all_atk_info.len(), 1);
        assert_eq!(stats_in_game.all_atk_info[0].atk_name, "Test Attack");
        assert_eq!(stats_in_game.all_atk_info[0].nb_use, 2);
        assert_eq!(
            stats_in_game.all_atk_info[0]
                .totals_by_target
                .get("Target1"),
            Some(&AccumulatedAtkInfo {
                total_full_heal: 200,
                total_full_dmg: 0,
                total_real_dmg: 0,
                total_real_heal: 100,
            })
        );
    }
}
