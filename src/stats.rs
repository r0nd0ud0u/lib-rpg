use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct TxRx {
    #[serde(rename = "Tx-rx-size")]
    tx_rx_size: u64,
    #[serde(rename = "Tx-rx-type")]
    tx_rx_type: u64,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct Attribute {
    #[serde(rename = "Current")]
    current: u32,
    #[serde(default)]
    current_raw: u32,
    #[serde(rename = "Max")]
    max: u32,
    #[serde(default)]
    max_raw: u32,
    #[serde(default)]
    buf_effect_value: u32,
    #[serde(default)]
    buf_effect_percent: u32,
    #[serde(default)]
    buf_equip_value: u32,
    #[serde(default)]
    buf_equip_percent: u32,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Stats {
    #[serde(rename = "Aggro")]
    aggro: Vec<Attribute>,

    #[serde(default, rename = "Aggro rate")]
    aggro_rate: Vec<Attribute>,

    #[serde(default, rename = "Magic armor")]
    magic_armor: Vec<Attribute>,

    #[serde(default, rename = "Physical armor")]
    physical_armor: Vec<Attribute>,

    #[serde(default, rename = "Magic strength")]
    magic_strength: Vec<Attribute>,

    #[serde(default, rename = "Physical strength")]
    physical_strength: Vec<Attribute>,

    #[serde(default, rename = "HP")]
    hp: Vec<Attribute>,

    #[serde(default, rename = "Mana")]
    mana: Vec<Attribute>,

    #[serde(default, rename = "Vigor")]
    vigor: Vec<Attribute>,

    #[serde(default, rename = "Berseck")]
    berseck: Vec<Attribute>,

    #[serde(default, rename = "Berseck rate")]
    berseck_rate: Vec<Attribute>,

    #[serde(default, rename = "Speed")]
    speed: Vec<Attribute>,

    #[serde(default, rename = "Critical strike")]
    critical_strike: Vec<Attribute>,

    #[serde(default, rename = "Dodge")]
    dodge: Vec<Attribute>,

    #[serde(default, rename = "HP regeneration")]
    regeneration_hp: Vec<Attribute>,

    #[serde(default, rename = "Mana regeneration")]
    regeneration_mana: Vec<Attribute>,

    #[serde(default, rename = "Vigor regeneration")]
    regeneration_vigor: Vec<Attribute>,

    #[serde(default, rename = "Speed regeneration")]
    regeneration_speed: Vec<Attribute>,
}
