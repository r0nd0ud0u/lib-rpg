use crate::common::effect_const::TARGET_ENNEMY;

#[cxx::bridge]
pub mod ffi {
    extern "Rust" {
        type ExtendedCharacter;
        /// Constructor
        pub fn try_new_ext_character() -> Box<ExtendedCharacter>;
        /// Getters
        pub fn get_is_random_target(&self) -> bool;
        pub fn get_is_heal_atk_blocked(&self) -> bool;
        /// Setters
        pub fn set_is_random_target(&mut self, value: bool);
        pub fn set_is_heal_atk_blocked(&mut self, value: bool);
        /// Static
        pub fn get_char_effect_value(target: &str) -> u8;
        pub fn get_coeffsign_effect_value(target: &str) -> i8;

    }
}

#[derive(Default, Debug, Clone)]
pub struct ExtendedCharacter {
    pub is_random_target: bool,
    pub is_heal_atk_blocked: bool,
}

pub fn try_new_ext_character() -> Box<ExtendedCharacter> {
    Box::<ExtendedCharacter>::default()
}

/// Static
///
pub fn get_char_effect_value(target: &str) -> u8 {
    let mut sign: u8 = b'+';
    if target == TARGET_ENNEMY {
        sign = b'-';
    }
    sign
}

///
pub fn get_coeffsign_effect_value(target: &str) -> i8 {
    let mut coeff: i8 = 1;
    if target == TARGET_ENNEMY {
        coeff = -1;
    }
    coeff
}

impl ExtendedCharacter {
    /// Getters
    pub fn get_is_random_target(&self) -> bool {
        self.is_random_target
    }
    pub fn get_is_heal_atk_blocked(&self) -> bool {
        self.is_heal_atk_blocked
    }
    /// Setters
    pub fn set_is_random_target(&mut self, value: bool) {
        self.is_random_target = value;
    }
    pub fn set_is_heal_atk_blocked(&mut self, value: bool) {
        self.is_heal_atk_blocked = value;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character::{get_char_effect_value, get_coeffsign_effect_value},
        common::effect_const::TARGET_ENNEMY,
    };

    #[test]
    fn unit_get_char_effect_value() {
        let result = get_char_effect_value(TARGET_ENNEMY);
        assert_eq!(b'-', result);

        let result = get_char_effect_value("OUPS");
        assert_eq!(b'+', result);
    }

    #[test]
    fn unit_get_coeffsign_effect_value() {
        let result = get_coeffsign_effect_value(TARGET_ENNEMY);
        assert_eq!(-1, result);

        let result = get_coeffsign_effect_value("OUPS");
        assert_eq!(1, result);
    }
}
