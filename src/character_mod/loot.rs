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

    pub fn format_loot(&self) -> String {
        format!(
            "{} ({}), Rank: {}, Level: {}, Classes: {}",
            self.name,
            match self.kind {
                LootType::Equipment => "Equipment",
                LootType::Consumable => "Consumable",
                LootType::Material => "Material",
                LootType::Currency => "Currency",
            },
            self.rank.to_str(),
            self.level,
            self.format_classes()
        )
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

    #[test]
    fn test_format_loot() {
        let loot = Loot {
            name: "Test Loot".to_string(),
            kind: LootType::Consumable,
            rank: Rank::Intermediate,
            level: 5,
            classes: vec![Class::Healer],
        };
        assert_eq!(
            loot.format_loot(),
            "Test Loot (Consumable), Rank: Intermediate, Level: 5, Classes: Healer"
        );
    }
}
