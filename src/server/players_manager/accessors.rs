use crate::character_mod::character::Character;

use super::PlayerManager;

impl PlayerManager {
    /// Get the number of active heroes with the given name
    pub fn get_nb_of_active_heroes_by_name(&self, db_full_name: &str) -> usize {
        self.active_heroes
            .iter()
            .filter(|c| c.db_full_name == db_full_name)
            .count()
    }

    /// Get the number of active bosses with the given name
    pub fn get_nb_of_active_bosses_by_name(&self, db_full_name: &str) -> usize {
        self.active_bosses
            .iter()
            .filter(|c| c.db_full_name == db_full_name)
            .count()
    }

    pub fn get_mut_active_character(&mut self, id_name: &str) -> Option<&mut Character> {
        if let Some(hero) = self.active_heroes.iter_mut().find(|c| c.id_name == id_name) {
            return Some(hero);
        }
        if let Some(boss) = self.active_bosses.iter_mut().find(|c| c.id_name == id_name) {
            return Some(boss);
        }
        None
    }

    pub fn get_active_character(&self, id_name: &str) -> Option<&Character> {
        if let Some(hero) = self.get_active_hero_character(id_name) {
            return Some(hero);
        }
        if let Some(boss) = self.get_active_boss_character(id_name) {
            return Some(boss);
        }
        None
    }

    pub fn modify_active_character(&mut self, id_name: &str) {
        let pl = self.current_player.clone();
        if let Some(hero) = self.get_mut_active_hero_character(id_name) {
            *hero = pl; // Modify the value inside self.active_heroes
        } else if let Some(boss) = self.get_mut_active_boss_character(id_name) {
            *boss = pl;
        }
    }

    pub fn get_mut_active_hero_character(&mut self, id_name: &str) -> Option<&mut Character> {
        self.active_heroes.iter_mut().find(|c| c.id_name == id_name)
    }

    pub fn get_mut_active_boss_character(&mut self, id_name: &str) -> Option<&mut Character> {
        self.active_bosses.iter_mut().find(|c| c.id_name == id_name)
    }

    pub fn get_active_hero_character(&self, id_name: &str) -> Option<&Character> {
        self.active_heroes.iter().find(|c| c.id_name == id_name)
    }

    pub fn get_active_boss_character(&self, id_name: &str) -> Option<&Character> {
        self.active_bosses.iter().find(|c| c.id_name == id_name)
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::testing_all_characters;

    #[test]
    fn unit_get_mut_active_character() {
        let mut pl = testing_all_characters::testing_pm();
        assert!(pl.get_mut_active_character("test_#1").is_some());
        assert!(pl.get_mut_active_character("test_boss1_#1").is_some());
        assert!(pl.get_mut_active_character("unknown").is_none());
    }

    #[test]
    fn unit_get_active_character() {
        let pl = testing_all_characters::testing_pm();
        assert!(pl.get_active_character("test_#1").is_some());
        assert!(pl.get_active_character("test_boss1_#1").is_some());
        assert!(pl.get_active_character("unknown").is_none());
    }

    #[test]
    fn unit_get_nb_of_active_heroes_by_name() {
        let pl = testing_all_characters::testing_pm();
        assert_eq!(1, pl.get_nb_of_active_heroes_by_name("test"));
        assert_eq!(0, pl.get_nb_of_active_heroes_by_name("unknown"));
    }

    #[test]
    fn unit_get_nb_of_active_bosses_by_name() {
        let pl = testing_all_characters::testing_pm();
        assert_eq!(1, pl.get_nb_of_active_bosses_by_name("test_boss1"));
        assert_eq!(0, pl.get_nb_of_active_bosses_by_name("unknown"));
    }
}
