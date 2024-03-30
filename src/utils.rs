use rand::Rng;

#[cxx::bridge]
mod utilsExtern {
    extern "Rust" {
        fn build_effect_name(raw_effect: &str, stats_name: &str) -> String;
        fn get_random_nb(min: i64, max: i64) -> i64;
    }
}

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

/// Returns a random number between min and max
pub fn get_random_nb(min: i64, max: i64) -> i64 {
    let mut rng = rand::thread_rng();
    // +1 is necessariy otherwise max is not included
    rng.gen_range(min..max + 1)
}

#[cfg(test)]
mod tests {
    use crate::utils::build_effect_name;

    use super::get_random_nb;

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
    fn unit_get_random_nb_works() {
        let result = get_random_nb(0, 100);
        assert!(result >= 0);
        assert!(result <= 100);
    }
}
