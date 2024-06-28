use byondapi::prelude::*;
use std::cell::RefCell;

use crate::_compat::setup_panic_handler;

#[ouroboros::self_referencing]
struct Map {
    map_data: String,
    #[borrows(map_data)]
    #[covariant]
    parsed_data: (dmm_lite::MapInfo, dmm_lite::MapData<'this>),
}

thread_local! {
    pub static PARSED_MAPS: RefCell<Vec<Map>> = const { RefCell::new(vec![]) };
}

pub mod _compat;
pub mod parse;

#[byondapi::bind]
/// This function empties out the cached map data
pub fn _bapidmm_clear_map_data() {
    setup_panic_handler();
    let _ = PARSED_MAPS.replace(vec![]);
    Ok(ByondValue::null())
}
