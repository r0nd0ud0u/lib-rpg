/// Define the kind of target
pub mod target_const {
    pub const TARGET_ENNEMY: &str = "Ennemy";
    pub const TARGET_ALLY: &str = "Ally";
}

/// Define all the stats of a character you can decode from JSON format
pub mod stats_const {
    pub const HP: &str = "HP";
    pub const MANA: &str = "Mana";
    pub const VIGOR: &str = "Vigor";
    pub const BERSECK: &str = "Berserk";
    pub const PHYSICAL_ARMOR: &str = "Physical armor";
    pub const MAGICAL_ARMOR: &str = "Magic armor";
    pub const PHYSICAL_POWER: &str = "Physical power";
    pub const MAGICAL_POWER: &str = "Magic power";
    pub const AGGRO: &str = "Aggro";
    pub const SPEED: &str = "Speed";
    pub const CRITICAL_STRIKE: &str = "Critical strike";
    pub const DODGE: &str = "Dodge";
    pub const HP_REGEN: &str = "HP regeneration";
    pub const MANA_REGEN: &str = "Mana regeneration";
    pub const VIGOR_REGEN: &str = "Vigor regeneration";
    pub const BERSECK_RATE: &str = "Berserk rate";
    pub const AGGRO_RATE: &str = "Aggro rate";
    pub const SPEED_REGEN: &str = "Speed regeneration";
}

/// Defines all the keys except stats you can decode from the JSON input
pub mod character_json_key {
    pub const STANDARD_CLASS: &str = "standard";
    pub const IS_BLOCKING_ATK: &str = "is-blocking-atk";
    pub const IS_CRIT_HEAL_AFTER_CRIT: &str = "is_crit_heal_after_crit";
    pub const IS_DAMAGE_TX_HEAL_NEEDY_ALLY: &str = "is_damage_tx_heal_needy_ally";
    pub const IS_FIRST_ROUND: &str = "is_first_round";
    pub const IS_HEAL_ATK_BLOCKED: &str = "is_heal_atk_blocked";
    pub const IS_RANDOM_TARGET: &str = "is_random_target";
    pub const MAX_NB_ACTIONS_IN_ROUND: &str = "Max-nb-actions-in-round";
    pub const NB_ACTIONS_IN_ROUND: &str = "nb-actions-in-round";
    pub const COLOR: &str = "Color";
    pub const EXPERIENCE: &str = "Experience";
    pub const SHAPE: &str = "Shape";
    pub const LEVEL: &str = "Level";
    pub const NAME: &str = "Name";
    pub const SHORT_NAME: &str = "Short name";
    pub const PHOTO: &str = "Photo";
    pub const TX_RX: &str = "Tx-rx";
    pub const TYPE: &str = "TYPE";
}

pub mod character_const {
    pub const SPEED_THRESHOLD: u64 = 100;
    pub const NB_TURN_SUM_AGGRO: usize = 5;
}

pub mod effect_const {
    /// Effect to improve max value of a stat by percent (current value is updated by ratio)
    pub const EFFECT_IMPROVE_BY_PERCENT_CHANGE: &str = "Up par %";
    // Effect to improve max value of a stat by value (current value is updated by ratio)
    pub const EFFECT_IMPROVEMENT_STAT_BY_VALUE: &str = "Up par valeur";
    pub const EFFECT_BLOCK_HEAL_ATK: &str = "Bloque attaque de soin";
    pub const EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT: &str = "Up/down degats en %";
    pub const EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT: &str = "Up/down degats RX en %";
    pub const EFFECT_CHANGE_HEAL_RX_BY_PERCENT: &str = "Up/down heal RX en %";
    pub const EFFECT_CHANGE_HEAL_TX_BY_PERCENT: &str = "Up/down heal TX en %";
}

pub mod paths_const {
    use lazy_static::lazy_static;
    use std::path::{Path, PathBuf};

    lazy_static! {
        /// Not used yet
        pub static ref OFFLINE_ROOT: &'static Path = Path::new("offlines");
        /// Path for directory where all the JSON character files are stored.
        pub static ref OFFLINE_CHARACTERS: PathBuf = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/offlines/characters"));
    }
}
