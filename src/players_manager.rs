use std::path::Path;

use anyhow::{bail, Result};

use crate::{
    character::{Character, CharacterType},
    common::paths_const::OFFLINE_CHARACTERS,
    utils::list_files_in_dir,
};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct PlayerManager {
    pub all_heroes: Vec<Character>,
    pub all_bosses: Vec<Character>,
}

impl PlayerManager {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<PlayerManager> {
        let mut pl = PlayerManager {
            ..Default::default()
        };
        pl.load_all_characters(path)?;
        Ok(pl)
    }

    pub fn load_all_characters<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        match list_files_in_dir(&path) {
            Ok(list) => list
                .iter()
                .for_each(|path| match Character::try_new_from_json(path) {
                    Ok(c) => {
                        if c.kind == CharacterType::Hero {
                            self.all_heroes.push(c);
                        } else {
                            self.all_bosses.push(c);
                        }
                    }
                    Err(e) => println!("{:?} cannot be decoded: {}", path, e),
                }),
            Err(e) => bail!(
                "Files cannot be listed in {:#?}: {}",
                OFFLINE_CHARACTERS.as_os_str(),
                e
            ),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerManager;

    #[test]
    fn unit_try_new() {
        let pl = PlayerManager::try_new("tests/characters").unwrap();
        assert_eq!(1, pl.all_heroes.len());

        assert!(PlayerManager::try_new("").is_err());
    }
}
