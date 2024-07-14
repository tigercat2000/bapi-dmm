#![allow(clippy::too_many_arguments)]
//! This is a variant of bapidmm loading where the maploading generates a list of commands,
//! to execute separately from doing expensive operations.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use byondapi::prelude::*;
use eyre::eyre;
use tracy_full::{frame, zone};

/// This type is used to wrap a ByondValue in IncRef/DecRef
struct SmartByondValue {
    _internal: ByondValue,
}

impl From<ByondValue> for SmartByondValue {
    fn from(mut value: ByondValue) -> Self {
        value.increment_ref();
        SmartByondValue { _internal: value }
    }
}

impl Drop for SmartByondValue {
    fn drop(&mut self) {
        self._internal.decrement_ref()
    }
}

impl SmartByondValue {
    fn get_copy(&self) -> ByondValue {
        self._internal
    }
}

type SharedByondValue = Rc<SmartByondValue>;

enum Command<'s> {
    CreateArea { loc: SharedByondValue, key: &'s str },
    CreateTurf { loc: SharedByondValue, key: &'s str },
    CreateAtom { loc: SharedByondValue, key: &'s str },
    IncreaseBounds { new_bounds: (usize, usize, usize) },
}

#[derive(Default)]
struct CommandBuffer<'s> {
    created_areas: HashMap<&'s str, SharedByondValue>,
    known_types: HashMap<&'s str, SharedByondValue>,
    commands: Vec<Command<'s>>,
}

thread_local! {
    static COMMAND_BUFFER: RefCell<CommandBuffer> = RefCell::new(CommandBuffer::default());
}

use crate::{
    _compat::setup_panic_handler,
    helpers::{ParsedMapTranslationLayer, _bapi_helper_get_world_bounds, _bapi_helper_tick_check},
    ouroboros_impl_map::Map,
    PARSED_MAPS,
};

#[byondapi::bind]
pub fn _bapidmm_work_commandbuffer() {
    COMMAND_BUFFER.with_borrow_mut(|commands| {
        PARSED_MAPS.with_borrow(|maps| {
            while let Some(command) = commands.pop() {
                match command {
                    Command::CreateArea { loc, key } => todo!(),
                    Command::CreateTurf { loc, key } => todo!(),
                    Command::CreateAtom { loc, key } => todo!(),
                    Command::IncreaseBounds { new_bounds } => todo!(),
                }

                // Yield
                if _bapi_helper_tick_check()? {
                    return Ok(ByondValue::new_num(1.));
                }
            }
            Ok(ByondValue::new_num(0.))
        })
    })
}

#[byondapi::bind]
pub fn _bapidmm_load_map_buffered(
    parsed_map: ByondValue,
    x_offset: ByondValue,
    y_offset: ByondValue,
    z_offset: ByondValue,
    crop_map: ByondValue,
    no_changeturf: ByondValue,
    x_lower: ByondValue,
    x_upper: ByondValue,
    y_lower: ByondValue,
    y_upper: ByondValue,
    z_lower: ByondValue,
    z_upper: ByondValue,
    place_on_top: ByondValue,
    new_z: ByondValue,
) {
    setup_panic_handler();

    let mut parsed_map = ParsedMapTranslationLayer { parsed_map };
    let id = parsed_map.get_internal_index()?;
    let x_offset = x_offset.get_number()?;
    let y_offset = y_offset.get_number()?;
    let z_offset = z_offset.get_number()?;
    let crop_map = crop_map.get_bool()?;
    let no_changeturf = no_changeturf.get_bool()?;
    let x_lower = x_lower.get_number()?;
    let x_upper = x_upper.get_number()?;
    let y_lower = y_lower.get_number()?;
    let y_upper = y_upper.get_number()?;
    let z_lower = z_lower.get_number()?;
    let z_upper = z_upper.get_number()?;
    let place_on_top = place_on_top.get_bool()?;
    let new_z = new_z.get_bool()?;

    PARSED_MAPS.with_borrow(|r| {
        let internal_data = r
            .get(id as usize)
            .ok_or_else(|| eyre!("Bad internal index {id:#?}"))?;

        parsed_map.set_loading(true)?;

        // Load map
        match load_map_impl(
            &mut parsed_map,
            internal_data,
            (x_offset, y_offset, z_offset),
            crop_map,
            no_changeturf,
            (x_lower, y_lower, z_lower),
            (x_upper, y_upper, z_upper),
            place_on_top,
            new_z,
        ) {
            Ok(_) => {}
            Err(e) => {
                parsed_map.add_warning(format!("Loading failed due to error: {e:#}"))?;
                return Err(e);
            }
        }

        parsed_map.set_loading(false)?;

        frame!();
        Ok(ByondValue::new_num(1.))
    })
}

fn load_map_impl(
    parsed_map: &mut ParsedMapTranslationLayer,
    internal_data: &Map,
    offset: (f32, f32, f32),
    crop_map: bool,
    no_changeturf: bool,
    // These MUST be f32 because they can be INFINITY
    lower_bounds: (f32, f32, f32),
    upper_bounds: (f32, f32, f32),
    place_on_top: bool,
    new_z: bool,
) -> eyre::Result<()> {
    zone!("load_map_impl");
    let (metadata, (prefabs, blocks)) = internal_data.borrow_parsed_data();

    let key_len = parsed_map.get_key_len()?;
    let parsed_bounds = parsed_map.get_parsed_bounds()?;
    let world_bounds = _bapi_helper_get_world_bounds()?;

    Ok(())
}
