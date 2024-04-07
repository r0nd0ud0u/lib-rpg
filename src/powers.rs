#[cxx::bridge]
pub mod ffi {
    pub struct Powers {
        /// Enables the critical of the next heal atk after a critical on damage atk
        pub is_crit_heal_after_crit: bool,
        /// Enables the power to heal the most needy ally using damage tx of previous turn
        pub is_damage_tx_heal_needy_ally: bool, 
    }
    extern "Rust" {
    }
}
