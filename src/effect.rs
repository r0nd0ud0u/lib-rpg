#[derive(Default, Debug, Clone)]
pub struct EffectParam {
    /// Received
    pub effect: String,
    pub nb_turns: i64,
    pub sub_value_effect: i64,
    pub target: String,
    pub reach: String,
    pub stats_name: String,

    /// Processed
    pub updated: bool,
    pub is_magic_atk: bool,
    pub counter_turn: i64,
}
