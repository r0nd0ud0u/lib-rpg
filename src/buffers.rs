/// Returns: i64
/// Returns the buf/debuf on cur_value.
/// its type {percent, decimal} and the additional value
pub fn update_damage_by_buf(add_value: i64, is_percent: bool, cur_value: i64) -> i64 {
    if is_percent {
        // sign of cur_value taken into account
        cur_value * add_value / 100
    } else {
        let sign = if cur_value > 0 { 1 } else { -1 };
        sign * add_value
    }
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
    pub all_stat_name: Vec<String>,
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
    pub fn add_stat_name(&mut self, value: &str) {
        self.all_stat_name.push(value.to_string() + "\0");
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
    pub fn get_all_stat_name(&self) -> &Vec<String> {
        &self.all_stat_name
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
        assert_eq!(result, 10);

        // buffer , negative decimal value
        let result = update_damage_by_buf(-10, false, 20);
        assert_eq!(result, -10);

        // buffer , percent value
        let result = update_damage_by_buf(10, true, 100);
        assert_eq!(result, 10);

        // buffer , negative percent value
        let result = update_damage_by_buf(-10, true, 200);
        assert_eq!(result, -20);

        // negative amount
        let result = update_damage_by_buf(-10, false, -200);
        assert_eq!(result, 10);
        let result = update_damage_by_buf(-10, true, -200);
        assert_eq!(result, 20);
    }

    #[test]
    fn unit_update_heal_by_multi() {
        let result = update_heal_by_multi(10, 0);
        assert_eq!(0, result);

        let result = update_heal_by_multi(10, 10);
        assert_eq!(100, result);
    }
}
