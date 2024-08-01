use byondapi::prelude::*;
use std::{cell::RefCell, collections::HashMap};

use crate::_compat::setup_panic_handler;

type ResumeKey = usize;

pub mod _compat;
pub mod grid;
pub mod helpers;
pub mod load_buffer;
pub mod parse;

/// This self-referencing structure holds a loaded map file in memory plus many
/// views into the map file, in the form of `&str`s from dmm_lite's `winnow` parser
#[ouroboros::self_referencing]
#[derive(Debug)]
struct Map {
    map_data: String,
    #[borrows(map_data)]
    #[covariant]
    parsed_data: (dmm_lite::MapInfo, dmm_lite::MapData<'this>),
    #[borrows(map_data, parsed_data)]
    #[covariant]
    command_buffers: HashMap<ResumeKey, load_buffer::CommandBuffer<'this>>,
}

/// Note: We only ever access this from the main BYOND thread, which is also where our DLL is loaded
static mut PARSED_MAPS: RefCell<Vec<Map>> = const { RefCell::new(vec![]) };

#[byondapi::bind]
/// This function empties out the cached map data
pub fn _bapidmm_clear_map_data() {
    setup_panic_handler();
    let _ = unsafe { PARSED_MAPS.replace(vec![]) };
    Ok(ByondValue::null())
}
