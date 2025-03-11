use crate::{
    character::{Character, CharacterType},
    common::paths_const::OFFLINE_CHARACTERS,
    utils::list_files_in_dir,
};

#[derive(Default, Debug, Clone)]
pub struct PlayerManager {
    pub all_heroes: Vec<Character>,
    pub all_bosses: Vec<Character>,
}

impl PlayerManager {
    pub fn build() -> PlayerManager {
        let mut pl = PlayerManager {
            ..Default::default()
        };
        pl.load_all_characters();
        pl
    }
    pub fn load_all_characters(&mut self) {
        match list_files_in_dir(OFFLINE_CHARACTERS) {
            Ok(list) => list
                .iter()
                .for_each(|path| match Character::decode_json(path) {
                    Ok(c) => {
                        if c.kind == CharacterType::Hero {
                            self.all_heroes.push(c);
                        } else {
                            self.all_bosses.push(c);
                        }
                    }
                    Err(_) => println!("{} cannot be decoded", path),
                }),
            Err(_) => println!("Files cannot be listed in {}", OFFLINE_CHARACTERS),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerManager;

    #[test]
    fn unit_build() {
       let pl= PlayerManager::build(); 
       assert_eq!(1, pl.all_heroes.len());
    }
}
