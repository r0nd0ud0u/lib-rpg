use ffi::Shared;

#[cxx::bridge]
mod ffi {
    #[derive(Clone)]
    struct Shared {
        v: u32,
    }
    extern "Rust" {
        //fn rusty_cxxbridge_integer() -> i32;
        fn rusty_cxxbridge_vector()-> Vec<Shared>;
       //fn rusty_cxxbridge_vector() -> Shared;
    }
}

/* pub fn rusty_cxxbridge_integer() -> i32 {
    42
} */

pub fn rusty_cxxbridge_vector()-> Vec<Shared> {
  [Shared{ v: 32 }].to_vec()
    //Shared{ v: 32 }
}

#[no_mangle]
pub extern "C" fn rusty_extern_c_integer() -> i32 {
    322
}
