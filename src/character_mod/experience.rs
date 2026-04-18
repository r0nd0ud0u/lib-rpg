use crate::character_mod::rank::Rank;

pub fn build_experience(rank: &Rank, level: u64) -> u64 {
    match rank {
        Rank::Common => level * 100,
        Rank::Intermediate => level * 200,
        Rank::Advanced => level * 300,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character_mod::rank::Rank;

    #[test]
    fn test_build_experience() {
        assert_eq!(build_experience(&Rank::Common, 1), 100);
        assert_eq!(build_experience(&Rank::Common, 5), 500);
        assert_eq!(build_experience(&Rank::Intermediate, 1), 200);
        assert_eq!(build_experience(&Rank::Intermediate, 5), 1000);
        assert_eq!(build_experience(&Rank::Advanced, 1), 300);
        assert_eq!(build_experience(&Rank::Advanced, 5), 1500);
    }
}
