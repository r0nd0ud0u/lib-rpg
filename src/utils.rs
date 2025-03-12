use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io, path::{Path, PathBuf}};

/// * Returns the concatenation of effect str and stats str
/// * If the effect str name is empty => only the stats str
///* If the stats str name is empty => only the effect str
pub fn build_effect_name(raw_effect: &str, stats_name: &str, is_cpp: bool) -> String {
    let mut effect_name = "".to_string();
    if raw_effect.is_empty() && !stats_name.is_empty() {
        effect_name = stats_name.to_string();
    } else if !raw_effect.is_empty() && stats_name.is_empty() {
        effect_name = raw_effect.to_string();
    } else if !raw_effect.is_empty() && !stats_name.is_empty() {
        effect_name = format!("{}-{}", stats_name, raw_effect);
    }
    if is_cpp {
        effect_name.to_string() + "\0"
    } else {
        effect_name.to_string()
    }
}

pub fn list_files_in_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.path().is_file() {
            files.push(entry.path());
        }
    }

    Ok(files)
}

pub fn read_from_json<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> Result<T> {
    let content = fs::read_to_string(path)?;
    let value: T = serde_json::from_str(&content)?;
    Ok(value)
}

fn _write_to_json<P: AsRef<Path>, T: Serialize>(value: &T, path: P) -> Result<()> {
    let data = serde_json::to_string_pretty(value)?;
    fs::write(path, data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::utils::build_effect_name;

    use super::list_files_in_dir;

    #[test]
    fn unit_build_effect_name_works() {
        // case args not empty
        let mut str = build_effect_name("effect", "stats", false);
        assert_eq!("stats-effect", str);
        // case effect str empty
        str = build_effect_name("", "stats", false);
        assert_eq!("stats", str);
        // case stats empty
        str = build_effect_name("effect", "", false);
        assert_eq!("effect", str);
        // case both args empty
        str = build_effect_name("", "", false);
        assert!(str.is_empty());
    }

    #[test]
    fn unit_list_files_in_dir() {
        let all_files = list_files_in_dir(Path::new("./tests/characters"));
        let list = all_files.unwrap();
        assert_eq!(1, list.len());
        assert_eq!(list[0], Path::new("./tests/characters/test.json"))
    }
}
