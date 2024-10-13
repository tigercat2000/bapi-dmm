use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
};

use typed_arena::Arena;

use crate::ResumeKey;

pub static mut ARENA: OnceCell<Arena<String>> = OnceCell::new();
pub static mut PARSED_MAPS_ARENABASED: RefCell<Vec<ArenaMap>> = RefCell::new(vec![]);

/// OnceCell helper: Gives you a mutable reference to the string arena.
///
/// # Safety
///
/// This must only be called from the main thread.
pub unsafe fn get_arena<'s>() -> &'s mut Arena<String> {
    if let Some(x) = ARENA.get_mut() {
        x
    } else {
        let _ = ARENA.set(Arena::new());
        ARENA.get_mut().unwrap()
    }
}

pub struct ArenaMap<'s> {
    pub parsed_data: (dmm_lite::MapInfo, dmm_lite::MapData<'s>),
    pub command_buffers: HashMap<ResumeKey, crate::load::command_buffer::CommandBuffer<'s>>,
}
