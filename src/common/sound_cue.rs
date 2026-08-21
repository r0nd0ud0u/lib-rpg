use serde::{Deserialize, Serialize};

use crate::server::game_manager::ResultLaunchAttack;

/// Which sound a client should play in reaction to a game event. Pure
/// classification only — playback is a client/UI concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundCue {
    Hit,
    CriticalHit,
    Dodge,
    Block,
    Heal,
    /// A consumable (potion) was used — see `GameState::last_consumable_use`.
    Potion,
    /// Scenario cleared — `GameState.status == GameStatus::EndOfScenario`.
    Victory,
    /// All heroes defeated — `GameState.status == GameStatus::EndOfGame`.
    GameOver,
}

/// Classifies a `ResultLaunchAttack` into the sound cues it should trigger,
/// most-significant first, deduplicated. Dodge/block take priority over any
/// landed-effect cues since a dodged/blocked attack didn't otherwise connect.
pub fn classify_result_atk(ra: &ResultLaunchAttack) -> Vec<SoundCue> {
    if ra.all_dodging.iter().any(|d| d.is_dodging) {
        return vec![SoundCue::Dodge];
    }
    if ra.all_dodging.iter().any(|d| d.is_blocking) {
        return vec![SoundCue::Block];
    }

    let mut cues = Vec::new();
    for effect in &ra.new_game_atk_effects {
        let cue = if effect.effect_outcome.is_critical {
            SoundCue::CriticalHit
        } else if effect.effect_outcome.real_amount_tx > 0 {
            // real_amount_tx is the actual HP change: positive for a heal,
            // negative for damage (see character_mod::character's heal/damage
            // application, e.g. the HP-potion tests: "real_amount_tx is positive").
            SoundCue::Heal
        } else if effect.effect_outcome.real_amount_tx < 0 {
            SoundCue::Hit
        } else {
            continue;
        };
        if !cues.contains(&cue) {
            cues.push(cue);
        }
    }
    cues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        character_mod::effect::EffectOutcome,
        server::players_manager::{DodgeInfo, GameAtkEffect},
    };

    fn effect_with(is_critical: bool, real_amount_tx: i64) -> GameAtkEffect {
        GameAtkEffect {
            effect_outcome: EffectOutcome {
                is_critical,
                real_amount_tx,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn dodge_beats_everything_else() {
        let ra = ResultLaunchAttack {
            all_dodging: vec![DodgeInfo {
                name: "boss".to_owned(),
                is_dodging: true,
                is_blocking: true,
            }],
            new_game_atk_effects: vec![effect_with(true, 10)],
            ..Default::default()
        };
        assert_eq!(classify_result_atk(&ra), vec![SoundCue::Dodge]);
    }

    #[test]
    fn block_beats_a_landed_hit() {
        let ra = ResultLaunchAttack {
            all_dodging: vec![DodgeInfo {
                name: "boss".to_owned(),
                is_dodging: false,
                is_blocking: true,
            }],
            new_game_atk_effects: vec![effect_with(false, 10)],
            ..Default::default()
        };
        assert_eq!(classify_result_atk(&ra), vec![SoundCue::Block]);
    }

    #[test]
    fn critical_effect_yields_critical_hit() {
        let ra = ResultLaunchAttack {
            new_game_atk_effects: vec![effect_with(true, 25)],
            ..Default::default()
        };
        assert_eq!(classify_result_atk(&ra), vec![SoundCue::CriticalHit]);
    }

    #[test]
    fn positive_amount_yields_heal() {
        let ra = ResultLaunchAttack {
            new_game_atk_effects: vec![effect_with(false, 15)],
            ..Default::default()
        };
        assert_eq!(classify_result_atk(&ra), vec![SoundCue::Heal]);
    }

    #[test]
    fn negative_amount_yields_hit() {
        let ra = ResultLaunchAttack {
            new_game_atk_effects: vec![effect_with(false, -8)],
            ..Default::default()
        };
        assert_eq!(classify_result_atk(&ra), vec![SoundCue::Hit]);
    }

    #[test]
    fn empty_result_yields_no_cues() {
        let ra = ResultLaunchAttack::default();
        assert_eq!(classify_result_atk(&ra), Vec::new());
    }

    #[test]
    fn zero_amount_effect_is_skipped() {
        let ra = ResultLaunchAttack {
            new_game_atk_effects: vec![effect_with(false, 0)],
            ..Default::default()
        };
        assert_eq!(classify_result_atk(&ra), Vec::new());
    }
}
