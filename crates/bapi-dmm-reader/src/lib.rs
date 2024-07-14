use byondapi::prelude::*;
use std::cell::RefCell;

use crate::_compat::setup_panic_handler;

#[ouroboros::self_referencing]
/// This self-referencing structure holds a loaded map file in memory plus many
/// views into the map file, in the form of `&str`s from dmm_lite's `winnow` parser
struct Map {
    map_data: String,
    #[borrows(map_data)]
    #[covariant]
    parsed_data: (dmm_lite::MapInfo, dmm_lite::MapData<'this>),
}

thread_local! {
    /// Note on thread_locals: We only ever access this from the main BYOND thread, which is also where our DLL is loaded
    pub static PARSED_MAPS: RefCell<Vec<Map>> = const { RefCell::new(vec![]) };
}

pub mod _compat;
pub mod helpers;
pub mod load;
pub mod load_buffer;
pub mod parse;

#[byondapi::bind]
/// This function empties out the cached map data
pub fn _bapidmm_clear_map_data() {
    setup_panic_handler();
    let _ = PARSED_MAPS.replace(vec![]);
    Ok(ByondValue::null())
}
