use byondapi::prelude::*;

use crate::{
    _compat::setup_panic_handler,
    arena::{ARENA, PARSED_MAPS_ARENABASED},
};

type ResumeKey = usize;

pub mod _compat;
pub mod arena;
pub mod load;
pub mod parse;
pub mod random_map;

#[byondapi::bind]
/// This function empties out the cached map data
pub fn _bapidmm_clear_map_data() {
    setup_panic_handler();
    // This must be dropped FIRST or there will be a bunch of invalid refs
    let _ = unsafe { PARSED_MAPS_ARENABASED.replace(vec![]) };
    let _ = unsafe { ARENA.take() };
    Ok(ByondValue::null())
}
