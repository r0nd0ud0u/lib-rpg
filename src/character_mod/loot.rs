use crate::character_mod::{class::Class, rank::Rank};

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Loot {
    pub name: String,
    pub kind: LootType,
    pub rank: Rank,
    pub level: i64,
    pub classes: Vec<Class>,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LootType {
    #[default]
    Equipment,
    Consumable,
    Material,
    Currency,
}

impl Loot {
    pub fn format_classes(&self) -> String {
        self.classes
            .iter()
            .map(|class| class.to_str())
            .collect::<Vec<&str>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_classes() {
        let loot = Loot {
            name: "Test Loot".to_string(),
            kind: LootType::Equipment,
            rank: Rank::Common,
            level: 1,
            classes: vec![Class::Warrior, Class::Mage],
        };
        assert_eq!(loot.format_classes(), "Warrior, Mage");
    }
}
