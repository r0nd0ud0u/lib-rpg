/// Define the kind of target
pub mod all_target_const {
    pub const TARGET_ENNEMY: &str = "Ennemie";
    pub const TARGET_ALLY: &str = "Allié";
    pub const TARGET_ALL_HEROES: &str = "Tous les heroes";
    pub const TARGET_HIMSELF: &str = "Soi-même";
    pub const TARGET_ONLY_ALLY: &str = "Seulement les alliés";
}

pub mod reach_const {
    pub const INDIVIDUAL: &str = "Individuel";
    pub const ZONE: &str = "Zone";
}

/// Define all the stats of a character you can decode from JSON format
pub mod stats_const {
    pub const HP: &str = "HP";
    pub const MANA: &str = "Mana";
    pub const VIGOR: &str = "Vigor";
    pub const BERSERK: &str = "Berserk";
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
    pub const STANDARD_CLASS: &str = "Standard";
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
    pub const ULTIMATE_LEVEL: u64 = 13;
}

pub mod effect_const {
    /// Effect to improve max value of a stat by percent (current value is updated by ratio)
    pub const EFFECT_IMPROVE_MAX_BY_PERCENT_CHANGE: &str = "Up par %";
    /// Effect to improve max value of a stat by value (current value is updated by ratio)
    pub const EFFECT_IMPROVE_MAX_STAT_BY_VALUE: &str = "Up par valeur";
    pub const EFFECT_BLOCK_HEAL_ATK: &str = "Bloque attaque de soin";
    pub const EFFECT_CHANGE_MAX_DAMAGES_BY_PERCENT: &str = "Up/down degats en %";
    pub const EFFECT_CHANGE_DAMAGES_RX_BY_PERCENT: &str = "Up/down degats RX en %";
    pub const EFFECT_CHANGE_HEAL_RX_BY_PERCENT: &str = "Up/down heal RX en %";
    pub const EFFECT_CHANGE_HEAL_TX_BY_PERCENT: &str = "Up/down heal TX en %";

    /// Effect to improve current value of a stat by value
    pub const EFFECT_VALUE_CHANGE: &str = "Changement par valeur";
    /// Effect to improve current value of a stat by percent
    pub const EFFECT_PERCENT_CHANGE: &str = "Changement par %";
    /// Assess the amount of applies for a stat
    pub const EFFECT_REPEAT_AS_MANY_AS: &str = "Répète tant que possible";
    /// Effect to execute an atk with a decreasing success rate defined by a step on effect value
    pub const EFFECT_NB_DECREASE_ON_TURN: &str = "Decroissement pendant le tour";
    pub const EFFECT_NB_DECREASE_BY_TURN: &str = "Decroissement par tour";
    pub const CONDITION_ENNEMIES_DIED: &str = "Ennemis morts tours précédents";

    pub const EFFECT_NB_COOL_DOWN: &str = "Tours de recharge";
    pub const EFFECT_REINIT: &str = "Reinit";
    pub const EFFECT_DELETE_BAD: &str = "Supprime effet néfaste";
    pub const EFFECT_IMPROVE_HOTS: &str = "Boost chaque HOT de .. %";
    pub const EFFECT_BOOSTED_BY_HOTS: &str = "Boost l'effet par nb HOTS presents en %";
    pub const EFFECT_INTO_DAMAGE: &str = "% (stats) en dégâts";
    pub const EFFECT_NEXT_HEAL_IS_CRIT: &str = "Prochaine attaque heal est crit";
    pub const EFFECT_BUF_MULTI: &str = "Buf multi";
    pub const EFFECT_BUF_VALUE_AS_MUCH_AS_HEAL: &str = "Buf par valeur d'autant de PV";
}

pub mod paths_const {
    use lazy_static::lazy_static;
    use std::path::{Path, PathBuf};

    lazy_static! {
        /// Not used yet
        pub static ref OFFLINE_CHARACTERS: &'static Path = Path::new("characters");
        pub static ref OFFLINE_ATTACKS: &'static Path = Path::new("attack");
        /// Path for directory where all the JSON character files are stored.
        pub static ref OFFLINE_ROOT: PathBuf = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/offlines"));
        /// save/load games
        pub static ref GAMES_DIR: &'static Path = Path::new("games");
        pub static ref OFFLINE_EQUIPMENT: &'static Path = Path::new("equipment");
        pub static ref OFFLINE_LOOT_EQUIPMENT: &'static Path = Path::new("equipment/body");
        pub static ref OFFLINE_EFFECTS: &'static Path = Path::new("effects");
        pub static ref OFFLINE_GAMESTATE: &'static Path = Path::new("game_state");
        pub static ref GAME_STATE_STATS_IN_GAME: &'static Path = Path::new("/stats_in_game_{}.csv");
    }
}

pub mod attak_const {
    pub const COEFF_CRIT_DMG: f64 = 2.0;
    pub const COEFF_CRIT_STATS: f64 = 1.5;
}
