#[cxx::bridge]
pub mod ffi {

    extern "Rust" {
        type Buffers;
        /// Setters
        pub fn set_is_passive_enabled(&mut self, value: bool);
        pub fn set_buffers(&mut self, value: i64, is_percent: bool);
        pub fn set_stat_name(&mut self, value: &str);
        /// Getters
        pub fn get_value(&self) -> i64;
        pub fn get_is_percent(&self) -> bool;
        pub fn get_is_passive_enabled(&self) -> bool;
        pub fn get_stat_name(&self) -> String;
        /// Constructor
        pub fn buffers_new() -> Box<Buffers>;
        /// Static methods
        pub fn update_damage_by_buf(cur_value: i64, is_percent: bool, new_value: i64) -> i64;
        pub fn update_heal_by_multi(cur_value: i64, coeff_multi: i64) -> i64;
    }
}

pub fn buffers_new() -> Box<Buffers> {
    Box::<Buffers>::default()
}

/// Returns: i64
/// The returned damage is calculated according the current value of the damage
/// its type {percent, decimal} and the additional value
pub fn update_damage_by_buf(add_value: i64, is_percent: bool, cur_value: i64) -> i64 {
    let mut output = cur_value;
    if cur_value > 0 {
        if is_percent {
            output += output * add_value / 100;
        } else if output > 0 {
            output += add_value;
        }
    }
    output
}

/// Returns: i64
/// Multiply cur_value value by coeff_multi
pub fn update_heal_by_multi(cur_value: i64, coeff_multi: i64) -> i64 {
    cur_value * coeff_multi
}

#[derive(Default, Debug, Clone)]
pub struct Buffers {
    /// A buf can be passive, that is without being a change of value
    pub is_passive_enabled: bool,
    /// If it is active, it changes the value
    pub value: i64,
    pub is_percent: bool,
    /// Potentially, a buffer can be applied on a stat, otherwise empty
    pub stat_name: String,
}

impl Buffers {
    // Setters
    pub fn set_buffers(&mut self, value: i64, is_percent: bool) {
        self.value = value;
        self.is_percent = is_percent;
    }
    pub fn set_is_passive_enabled(&mut self, value: bool) {
        self.is_passive_enabled = value;
    }
    pub fn set_stat_name(&mut self, value: &str) {
        self.stat_name = value.to_string();
    }
    // Getters
    pub fn get_value(&self) -> i64 {
        self.value
    }
    pub fn get_is_percent(&self) -> bool {
        self.is_percent
    }
    pub fn get_is_passive_enabled(&self) -> bool {
        self.is_passive_enabled
    }
    pub fn get_stat_name(&self) -> String {
        self.stat_name.to_string() + "\0"
    }
}

#[cfg(test)]
mod tests {
    use crate::buffers::update_heal_by_multi;

    use super::update_damage_by_buf;

    #[test]
    pub fn unit_update_damage_by_buf() {
        // default buffer
        let result = update_damage_by_buf(0, false, 0);
        assert_eq!(result, 0);

        // buffer , decimal value
        let result = update_damage_by_buf(10, false, 20);
        assert_eq!(result, 30);

        // buffer , percent value
        let result = update_damage_by_buf(10, true, 100);
        assert_eq!(result, 110);
    }

    #[test]
    fn unit_update_heal_by_multi() {
        let result = update_heal_by_multi(10, 0);
        assert_eq!(0, result);

        let result = update_heal_by_multi(10, 10);
        assert_eq!(100, result);
    }
}
