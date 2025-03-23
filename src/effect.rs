use serde::{Deserialize, Serialize};

/// Define the parameters of an effect.
/// An effect can be enabled from an attack, a passive power or an object.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectParam {
    /// Received
    /// Name of the effect
    pub effect: String,
    /// Duration of the effect
    pub nb_turns: i64,
    /// TODO sub_value_effect
    pub sub_value_effect: i64,
    /// TODO target of the effect, ally or ennemy
    pub target: String,
    /// TODO, reach of the effect, zone or individual
    pub reach: String,
    /// Name of the targeted stat
    pub stats_name: String,

    /// Processed
    /// TODO
    pub updated: bool,
    /// TODO from a magical attack ?or is magical effect ?
    pub is_magic_atk: bool,
    /// Lasting turns
    pub counter_turn: i64,
}
