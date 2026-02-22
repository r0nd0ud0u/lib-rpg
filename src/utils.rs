use anyhow::Result;
use rand::Rng;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// * Returns the concatenation of effect str and stats str
/// * If the effect str name is empty => only the stats str
///* If the stats str name is empty => only the effect str
pub fn build_effect_name(raw_effect: &str, stats_name: &str) -> String {
    let mut effect_name = "".to_string();
    if raw_effect.is_empty() && !stats_name.is_empty() {
        effect_name = stats_name.to_string();
    } else if !raw_effect.is_empty() && stats_name.is_empty() {
        effect_name = raw_effect.to_string();
    } else if !raw_effect.is_empty() && !stats_name.is_empty() {
        effect_name = format!("{}-{}", stats_name, raw_effect);
    }
    effect_name.to_string()
}

pub fn list_files_in_dir<P: AsRef<Path>>(path: P) -> io::Result<Vec<PathBuf>> {
    // Normalize the path to ensure consistent behavior across platforms
    let normalized = normalize_cross_platform(path);

    let mut files = Vec::new();

    for entry in fs::read_dir(&normalized)? {
        let entry = entry?;
        if entry.path().is_file() {
            files.push(normalize_cross_platform(entry.path()));
        }
    }

    Ok(files)
}

pub fn normalize_cross_platform<P: AsRef<Path>>(path: P) -> PathBuf {
    let s = path.as_ref().to_string_lossy().to_string();
    #[cfg(windows)]
    let fixed = s.replace('/', std::path::MAIN_SEPARATOR.to_string().as_str());

    #[cfg(not(windows))]
    let fixed = s.replace('\\', std::path::MAIN_SEPARATOR.to_string().as_str());

    PathBuf::from(fixed)
}

pub fn list_dirs_in_dir<P: AsRef<Path>>(path: P) -> io::Result<Vec<PathBuf>> {
    // Normalize the path to ensure consistent behavior across platforms
    let normalized = normalize_cross_platform(path);

    let mut files = Vec::new();

    for entry in fs::read_dir(normalized)? {
        let entry = entry?;
        if entry.path().is_dir() {
            files.push(normalize_cross_platform(entry.path()));
        }
    }

    Ok(files)
}

pub fn read_from_json<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> Result<T> {
    let content = fs::read_to_string(path)?;
    let value: T = serde_json::from_str(&content)?;
    Ok(value)
}

pub(crate) fn write_to_json<P: AsRef<Path>, T: Serialize>(value: &T, path: P) -> Result<()> {
    let data = serde_json::to_string_pretty(value)?;
    fs::write(path, data)?;
    Ok(())
}

pub fn get_current_time_as_string() -> String {
    let now = chrono::Local::now();
    now.format("%Y-%m-%d-%H-%M-%S").to_string()
}

pub fn calc_ratio(val1: i64, val2: i64) -> f64 {
    if val2 > 0 {
        val1 as f64 / val2 as f64
    } else {
        1.0
    }
}

pub fn get_random_nb(min: i64, max: i64) -> i64 {
    let mut rng = rand::rng();
    rng.random_range(min..=max)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::utils::{build_effect_name, list_dirs_in_dir};

    use super::list_files_in_dir;

    #[test]
    fn unit_build_effect_name_works() {
        // case args not empty
        let mut str = build_effect_name("effect", "stats");
        assert_eq!("stats-effect", str);
        // case effect str empty
        str = build_effect_name("", "stats");
        assert_eq!("stats", str);
        // case stats empty
        str = build_effect_name("effect", "");
        assert_eq!("effect", str);
        // case both args empty
        str = build_effect_name("", "");
        assert!(str.is_empty());
    }

    #[test]
    fn unit_list_files_in_dir() {
        let all_files = list_files_in_dir(Path::new("./tests/offlines/characters"));
        let list = all_files.unwrap();
        assert!(list.len() > 1);
    }

    #[test]
    fn unit_list_dirs_in_dir() {
        let all_dirs = list_dirs_in_dir(PathBuf::from(".\\tests\\offlines"));
        let list = all_dirs.unwrap();
        assert!(list.len() > 0);

        let all_dirs = list_dirs_in_dir(Path::new("./tests/offlines"));
        let list = all_dirs.unwrap();
        assert!(list.len() > 0);
    }

    #[test]
    fn unit_normalize_cross_platform() {
        let path = Path::new("some/path/to/file");
        let normalized = super::normalize_cross_platform(path);
        #[cfg(windows)]
        assert_eq!(normalized.to_str().unwrap(), "some\\path\\to\\file");
        #[cfg(not(windows))]
        assert_eq!(normalized.to_str().unwrap(), "some/path/to/file");
    }

    #[test]
    fn unit_get_current_time_as_string() {
        let time_str = super::get_current_time_as_string();
        assert_eq!(19, time_str.len());
    }

    #[test]
    fn unit_calc_ratio() {
        let ratio = super::calc_ratio(10, 20);
        assert_eq!(0.5, ratio);
        let ratio = super::calc_ratio(10, 0);
        assert_eq!(1.0, ratio);
    }
}
