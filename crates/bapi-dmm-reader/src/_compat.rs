/// This is all stuff for compatibility playing nice with BYOND
/// such as error logging, panic handling, binding generation
use byondapi::prelude::*;
use std::{fs::OpenOptions, io::Write};

pub fn write_log<T: AsRef<[u8]>>(x: T) {
    if let Ok(mut f) = OpenOptions::new()
        .append(true)
        .create(true)
        .open("./rust_log.txt")
    {
        let _ = f.write_all(x.as_ref());
    }
}

pub fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        write_log(format!("Panic {:#?}", info));
    }))
}

#[byondapi::bind]
/// Returns "10" if loaded correctly
pub fn _bapidmm_test_connection() {
    Ok(ByondValue::new_num(10f32))
}

#[ignore = "Generates bindings in current directory"]
#[test]
fn generate_binds() {
    byondapi::generate_bindings(env!("CARGO_CRATE_NAME"));
}
