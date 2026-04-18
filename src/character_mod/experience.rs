use crate::character_mod::{class::Class, rank::Rank};

/// Experience gained by defeating a boss of the given rank and level.
pub fn build_experience(rank: &Rank, level: u64) -> u64 {
    match rank {
        Rank::Common => level * 100,
        Rank::Intermediate => level * 200,
        Rank::Advanced => level * 300,
    }
}

/// Class multiplier for exp required to reach the next level.
/// Returns (numerator, denominator) for integer arithmetic.
/// Standard/Berserker = ×1.0, Warrior = ×1.2, Healer = ×1.3, Mage = ×1.5
fn class_exp_factor(class: &Class) -> (u64, u64) {
    match class {
        Class::Standard | Class::Berserker => (10, 10),
        Class::Warrior => (12, 10),
        Class::Healer => (13, 10),
        Class::Mage => (15, 10),
    }
}

/// Experience required by a hero to reach the next level,
/// derived from the hero's rank, class, and current level.
pub fn build_exp_to_next_level(rank: &Rank, class: &Class, level: u64) -> u64 {
    let base = build_experience(rank, level);
    let (num, den) = class_exp_factor(class);
    base * num / den
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character_mod::{class::Class, rank::Rank};

    #[test]
    fn test_build_experience() {
        assert_eq!(build_experience(&Rank::Common, 1), 100);
        assert_eq!(build_experience(&Rank::Common, 5), 500);
        assert_eq!(build_experience(&Rank::Intermediate, 1), 200);
        assert_eq!(build_experience(&Rank::Intermediate, 5), 1000);
        assert_eq!(build_experience(&Rank::Advanced, 1), 300);
        assert_eq!(build_experience(&Rank::Advanced, 5), 1500);
    }

    #[test]
    fn test_build_exp_to_next_level_standard() {
        // Standard/Berserker: same as build_experience base
        assert_eq!(
            build_exp_to_next_level(&Rank::Common, &Class::Standard, 1),
            100
        );
        assert_eq!(
            build_exp_to_next_level(&Rank::Common, &Class::Berserker, 1),
            100
        );
        assert_eq!(
            build_exp_to_next_level(&Rank::Intermediate, &Class::Standard, 1),
            200
        );
        assert_eq!(
            build_exp_to_next_level(&Rank::Advanced, &Class::Standard, 5),
            1500
        );
    }

    #[test]
    fn test_build_exp_to_next_level_class_factors() {
        // Common rank, level 1, base = 100
        assert_eq!(
            build_exp_to_next_level(&Rank::Common, &Class::Warrior, 1),
            120
        );
        assert_eq!(
            build_exp_to_next_level(&Rank::Common, &Class::Healer, 1),
            130
        );
        assert_eq!(build_exp_to_next_level(&Rank::Common, &Class::Mage, 1), 150);
    }

    #[test]
    fn test_build_exp_to_next_level_scales_with_level() {
        // Common Mage: base * 1.5, level 5 → 500 * 1.5 = 750
        assert_eq!(build_exp_to_next_level(&Rank::Common, &Class::Mage, 5), 750);
        // Intermediate Warrior: base 200, level 2 → 400 * 1.2 = 480
        assert_eq!(
            build_exp_to_next_level(&Rank::Intermediate, &Class::Warrior, 2),
            480
        );
    }
}
