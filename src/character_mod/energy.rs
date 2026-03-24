#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct Energy {
    pub kind: EnergyKind,
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub enum EnergyKind {
    #[default]
    Mana,
    Vigor,
    Berserk,
}
