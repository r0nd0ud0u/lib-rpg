pub mod effect_const {
    pub const TARGET_ENNEMY: &str = "Ennemie";
}

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
    pub const REGEN_HP: &str = "HP regeneration";
    pub const REGEN_MANA: &str = "Mana regeneration";
    pub const REGEN_VIGOR: &str = "Vigor regeneration";
    pub const RATE_BERSECK: &str = "Berserk rate";
    pub const RATE_AGGRO: &str = "Aggro rate";
    pub const REGEN_SPEED: &str = "Speed regeneration";
}

pub mod character_const {
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

pub mod paths_const {
    use std::path::Path;
    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref OFFLINE_ROOT: &'static Path = Path::new("./offlines");
        pub static ref OFFLINE_CHARACTERS: &'static Path = Path::new("./offlines/characters");
    }
}
