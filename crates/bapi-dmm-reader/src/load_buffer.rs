#![allow(clippy::too_many_arguments)]
#![allow(unused_variables)]
//! This is a variant of bapidmm loading where the maploading generates a list of commands,
//! to execute separately from doing expensive operations.

use std::{collections::HashMap, rc::Rc};

use byondapi::{
    map::{byond_locatexyz, ByondXYZ},
    prelude::*,
};
use dmm_lite::prefabs::{Literal, Prefab};
use eyre::eyre;
use tracy_full::{frame, zone};

/// This type is used to wrap a ByondValue in IncRef/DecRef
pub struct SmartByondValue {
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
    pub fn get_temp_ref(&self) -> ByondValue {
        self._internal
    }
}

pub type SharedByondValue = Rc<SmartByondValue>;

pub enum Command<'s> {
    CreateArea {
        loc: SharedByondValue,
        prefab: &'s Prefab<'s>,
        new_z: bool,
    },
    CreateTurf {
        loc: SharedByondValue,
        prefab: &'s Prefab<'s>,
        no_changeturf: bool,
        place_on_top: bool,
    },
    CreateAtom {
        loc: SharedByondValue,
        prefab: &'s Prefab<'s>,
    },
}

#[derive(Default)]
pub struct CommandBuffer<'s> {
    pub created_areas: HashMap<&'s str, SharedByondValue>,
    pub known_types: HashMap<&'s str, SharedByondValue>,
    pub commands: Vec<Command<'s>>,
}

use crate::{
    _compat::setup_panic_handler,
    helpers::{
        ParsedMapTranslationLayer, _bapi_add_turf_to_area, _bapi_apply_preloader,
        _bapi_create_or_get_area, _bapi_create_turf, _bapi_handle_area_contain,
        _bapi_helper_get_world_bounds, _bapi_helper_get_world_type_area,
        _bapi_helper_get_world_type_turf, _bapi_helper_text2file, _bapi_helper_text2path,
        _bapi_helper_tick_check, _bapi_setup_preloader,
    },
    ouroboros_impl_map::Map,
    PARSED_MAPS,
};

const MIN_PAUSE: usize = 100;

#[byondapi::bind]
pub fn _bapidmm_work_commandbuffer(parsed_map: ByondValue, resume_key: ByondValue) {
    zone!("_bapidmm_work_commandbuffer");
    setup_panic_handler();
    let mut parsed_map = ParsedMapTranslationLayer { parsed_map };
    let id = parsed_map.get_internal_index()? as usize;
    let resume_key = resume_key.get_number()? as usize;

    zone!("borrow parsed_map");
    PARSED_MAPS.with_borrow_mut(|maps| {
        let internal_data = maps
            .get_mut(id)
            .ok_or_else(|| eyre!("Bad internal index {id:#?}"))?;

        zone!("borrow internal_data");
        let mut minimum_pause_counter = 0;
        internal_data.with_mut(|all_fields| {
            let command_buffers_map = all_fields.command_buffers;
            let (metadata, (prefabs, _blocks)) = all_fields.parsed_data;

            zone!("lookup our buffer");
            if let Some(our_command_buffer) = command_buffers_map.get_mut(&resume_key) {
                zone!("command loop");
                while let Some(command) = our_command_buffer.commands.pop() {
                    match command {
                        Command::CreateArea { loc, prefab, new_z } => {
                            zone!("Commmand::CreateArea");

                            let area = if let Some(area) =
                                our_command_buffer.created_areas.get_mut(prefab.0)
                            {
                                area
                            } else {
                                zone!("new area creation");
                                let area = _bapi_create_or_get_area(prefab.0)?;
                                let area = Rc::new(SmartByondValue::from(area));
                                our_command_buffer.created_areas.insert(prefab.0, area);
                                // This can't possibly fail, I hope
                                our_command_buffer.created_areas.get_mut(prefab.0).unwrap()
                            };

                            if !new_z {
                                _bapi_handle_area_contain(loc.get_temp_ref(), area.get_temp_ref())?;
                            }
                            _bapi_add_turf_to_area(area.get_temp_ref(), loc.get_temp_ref())?;
                        }
                        Command::CreateTurf {
                            loc,
                            prefab,
                            no_changeturf,
                            place_on_top,
                        } => {
                            zone!("Commmand::CreateTurf");
                            create_turf(&mut parsed_map, loc, prefab, place_on_top, no_changeturf)?;
                        }
                        Command::CreateAtom { loc, prefab } => {
                            zone!("Commmand::CreateAtom");
                            create_movable(
                                &mut parsed_map,
                                &mut our_command_buffer.known_types,
                                loc,
                                prefab,
                            )?;
                        }
                    }
                    minimum_pause_counter += 1;

                    // Yield
                    if minimum_pause_counter % MIN_PAUSE == 0 && _bapi_helper_tick_check()? {
                        minimum_pause_counter = 0;
                        return Ok(ByondValue::new_num(1.));
                    }
                }

                // Clean up after ourselves
                if our_command_buffer.commands.is_empty() {
                    zone!("cleanup");
                    command_buffers_map.remove(&resume_key);
                }
            }

            zone!("set_loading false and return 0");
            parsed_map.set_loading(false)?;

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

    PARSED_MAPS.with_borrow_mut(|r| {
        let internal_data = r
            .get_mut(id as usize)
            .ok_or_else(|| eyre!("Bad internal index {id:#?}"))?;

        parsed_map.set_loading(true)?;

        // Load map
        let ret = match generate_command_buffer(
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
            Ok(val) => Ok(val),
            Err(e) => {
                parsed_map.add_warning(format!("Loading failed due to error: {e:#}"))?;
                Err(e)
            }
        };

        frame!();
        ret
    })
}

fn generate_command_buffer(
    parsed_map: &mut ParsedMapTranslationLayer,
    internal_data: &mut Map,
    offset: (f32, f32, f32),
    crop_map: bool,
    no_changeturf: bool,
    // These MUST be f32 because they can be INFINITY
    lower_bounds: (f32, f32, f32),
    upper_bounds: (f32, f32, f32),
    place_on_top: bool,
    new_z: bool,
) -> eyre::Result<ByondValue> {
    zone!("generate_command_buffer");

    internal_data.with_mut(|fields| {
        let (metadata, (prefabs, blocks)) = fields.parsed_data;
        let command_buffers = fields.command_buffers;
        let resume_key = command_buffers.len() + 1;

        let mut our_command_buffer = CommandBuffer::default();

        let key_len = parsed_map.get_key_len()?;
        let parsed_bounds = parsed_map.get_parsed_bounds()?;
        let world_bounds = _bapi_helper_get_world_bounds()?;

        let world_turf = _bapi_helper_get_world_type_turf()?;
        let world_area = _bapi_helper_get_world_type_area()?;

        let space_key: Option<&str> = if no_changeturf {
            prefabs.iter().find_map(|(key, prefab_list)| {
                if prefab_list.len() != 2 {
                    return None;
                }
                match prefab_list[0] {
                    (turf, None) if turf == world_turf => {}
                    _ => return None,
                }
                match prefab_list[1] {
                    (area, None) if area == world_area => {}
                    _ => return None,
                }
                Some(*key)
            })
        } else {
            None
        };

        // We know bounds ahead of time so we
        let mut no_afterchange = no_changeturf;
        if parsed_bounds.5 + (offset.2 as usize) - 1 > world_bounds.2 {
            // z expansion
            if !no_changeturf {
                parsed_map.add_warning("Z-level expansion occurred without no_changeturf set, this may cause problems when /turf/AfterChange is called, and therefore ChangeTurf will NOT be called")?;
                no_afterchange = true; // force no_afterchange
            }
        }

        // (minx, miny, minz, maxx, maxy, maxz)
        // starts at (1, 1, 1)
        let mut bounds = (usize::MAX, usize::MAX, usize::MAX, 1, 1, 1);

        for (bottom_left, block) in blocks {
            // We have to reverse and THEN enumerate this to translate from
            // origin TOP left to origin BOTTOM left
            // and then reverse it again to do the correct iteration order
            for (map_y_offset, line) in block.iter().rev().enumerate().rev() {
                let turfs = separate_turfs(line, key_len as usize);
                for (map_x_offset, prefab_key) in turfs.enumerate() {
                    let relative_coord = (
                        bottom_left.0 + map_x_offset,
                        bottom_left.1 + map_y_offset,
                        bottom_left.2,
                    );

                    // Skip anything outside of our relative bounds
                    if float_exceeds_upper_bounds(relative_coord, upper_bounds) {
                        continue;
                    }
                    // for some reason, negative bounds are permitted?
                    if float_exceeds_lower_bounds(relative_coord, lower_bounds) {
                        continue;
                    }

                    // Calculate absolute position
                    // This is offset - 1 because (1,1,1) actually goes *at* offset
                    let exact_coord = (
                        relative_coord.0 + offset.0 as usize - 1,
                        relative_coord.1 + offset.1 as usize - 1,
                        relative_coord.2 + offset.2 as usize - 1,
                    );

                    // This will just guaranteed fail to locate a turf
                    if exceeds_lower_bounds(exact_coord, (1, 1, 1)) {
                        parsed_map.add_warning(format!(
                            "Bad map coord (tries to spawn in negative space): {exact_coord:#?}"
                        ))?;
                        continue;
                    }

                    // Expand map if necessary
                    if exceeds_upper_bounds(exact_coord, world_bounds) {
                        if crop_map {
                            continue;
                        } else {
                            parsed_map.expand_map(exact_coord, new_z, offset.2)?;
                        }
                    }

                    // Locate the turf at the xyz
                    let turf = byond_locatexyz(ByondXYZ::with_coords((
                        exact_coord.0 as i16,
                        exact_coord.1 as i16,
                        exact_coord.2 as i16,
                    )))?;

                    if turf.is_null() {
                        // This should be unreachable
                        // We check
                        // - coord isn't < (1, 1, 1)
                        // - coord isn't > world_bounds
                        // And either continue and skip the rest of this
                        // Or we expand the world to fit.
                        // Both of which should mean that our locate call never fails... but just to be safe, we never want to spawn stuff in nullspace.
                        // Note: WE CANNOT ERROR HERE.
                        // If we error here, the map will have only partially loaded.
                        parsed_map.add_warning(format!(
                            "Failed to locate turf at: {exact_coord:#?}, skipping"
                        ))?;
                        continue;
                    }

                    let turf = Rc::new(SmartByondValue::from(turf));

                    if Some(prefab_key) == space_key && no_afterchange {
                        continue;
                    }

                    if let Some(prefab) = prefabs.get(prefab_key) {
                        // DMM prefab require that all prefab lists end with one /turf, and then one /area.
                        if prefab.len() < 2 {
                            parsed_map.add_warning(format!(
                                "Prefab {prefab_key:#?} is too short, violating requirement for /turf and /area!"
                            ))?;
                            continue;
                        }

                        // This is the point where we are committed, we are GOING to put something at this coord
                        // Accordingly, this is where we calculate bounds
                        bounds.0 = bounds.0.min(exact_coord.0);
                        bounds.1 = bounds.1.min(exact_coord.1);
                        bounds.2 = bounds.2.min(exact_coord.2);
                        bounds.3 = bounds.3.max(exact_coord.0);
                        bounds.4 = bounds.4.max(exact_coord.1);
                        bounds.5 = bounds.5.max(exact_coord.2);

                        let mut prefab_list = prefab.iter().rev();
                        // Above check ensures that these cannot panic
                        let prefab_area = prefab_list.next().unwrap();
                        if !prefab_area.0.starts_with("/area") {
                            parsed_map.add_warning(format!(
                                "Prefab {prefab_key:#?} does not end in an area, instead ending in {prefab_area:#?}!"
                            ))?;
                            continue;
                        }
                        if !prefab_area.0.starts_with("/area/template_noop") {
                            zone!("generating CreateArea");
                            our_command_buffer.commands.push(Command::CreateArea { loc: turf.clone(), prefab: prefab_area, new_z });
                        }

                        let prefab_turf = prefab_list.next().unwrap();
                        if !prefab_turf.0.starts_with("/turf") {
                            parsed_map.add_warning(format!(
                                    "Prefab {prefab_key:#?} does not second-end in a turf, instead ending in {prefab_turf:#?}!"
                                ))?;
                            continue;
                        }
                        if !prefab_turf.0.starts_with("/turf/template_noop") {
                            zone!("generating CreateTurf");
                            our_command_buffer.commands.push(Command::CreateTurf { loc: turf.clone(), prefab: prefab_turf, no_changeturf: no_afterchange, place_on_top  })
                        }

                        // We reverse it again after doing the turf and area
                        for instance in prefab_list.rev() {
                            // We allow these but warn about them
                            if !instance.0.starts_with("/obj") && !instance.0.starts_with("/mob") {
                                parsed_map.add_warning(
                                    format!(
                                        "Prefab {prefab_key:#?} has a strange element that we'll treat as a movable: {instance:#?}"
                                    ))?;
                            }
                            zone!("generating CreateAtom");
                            // Movables are easy
                            our_command_buffer.commands.push(Command::CreateAtom { loc: turf.clone(), prefab: instance });
                        }
                    } else {
                        // Note: Cannot hard error or map will fail to finish loading
                        // This is necessarily just a warning
                        parsed_map.add_warning(format!("Invalid prefab key: {prefab_key:#?}"))?;
                    }
                }
            }
        }

        parsed_map.set_bounds(bounds)?;
        command_buffers.insert(resume_key, our_command_buffer);

        Ok(ByondValue::new_num(resume_key as f32))
    })
}

fn create_turf(
    parsed_map: &mut ParsedMapTranslationLayer,
    turf: SharedByondValue,
    prefab_turf: &dmm_lite::prefabs::Prefab,
    place_on_top: bool,
    no_changeturf: bool,
) -> eyre::Result<ByondValue> {
    zone!("create_turf");
    let (path_text, vars) = prefab_turf;

    zone!("creating path string");
    let vars_list = convert_vars_list_to_byondlist(parsed_map, vars)?;

    _bapi_create_turf(
        turf.get_temp_ref(),
        path_text,
        vars_list,
        place_on_top,
        no_changeturf,
    )
}

fn create_movable<'s>(
    parsed_map: &mut ParsedMapTranslationLayer,
    path_cache: &mut HashMap<&'s str, SharedByondValue>,
    turf: SharedByondValue,
    obj: &'s dmm_lite::prefabs::Prefab,
) -> eyre::Result<ByondValue> {
    zone!("movable creation");
    let (path_text, vars) = obj;
    let path = if let Some(path) = path_cache.get(*path_text) {
        path
    } else {
        let path = _bapi_helper_text2path(path_text)?;
        let path = Rc::new(SmartByondValue::from(path));
        path_cache.insert(path_text, path);
        path_cache.get(path_text).unwrap()
    };

    if vars.is_some() {
        let vars_list = convert_vars_list_to_byondlist(parsed_map, vars)?;
        _bapi_setup_preloader(vars_list, path.get_temp_ref())?;
    }

    zone!("byond_new");
    let instance = ByondValue::builtin_new(path.get_temp_ref(), &[turf.get_temp_ref()])?;

    _bapi_apply_preloader(instance)?;

    Ok(instance)
}

pub fn separate_turfs(mut s: &str, n: usize) -> impl Iterator<Item = &'_ str> {
    assert_ne!(n, 0);
    std::iter::from_fn(move || {
        let index = s
            .char_indices()
            .nth(n)
            .map(|(index, _)| index)
            .unwrap_or(s.len());
        let (item, rest) = s.split_at(index);
        if item.is_empty() {
            None
        } else {
            s = rest;
            Some(item)
        }
    })
}

// Helpers
pub fn exceeds_upper_bounds(check: (usize, usize, usize), bounds: (usize, usize, usize)) -> bool {
    check.0 > bounds.0 || check.1 > bounds.1 || check.2 > bounds.2
}

pub fn exceeds_lower_bounds(check: (usize, usize, usize), bounds: (usize, usize, usize)) -> bool {
    check.0 < bounds.0 || check.1 < bounds.1 || check.2 < bounds.2
}

pub fn float_exceeds_upper_bounds(check: (usize, usize, usize), bounds: (f32, f32, f32)) -> bool {
    (check.0 as f32) > bounds.0 || (check.1 as f32) > bounds.1 || (check.2 as f32) > bounds.2
}

pub fn float_exceeds_lower_bounds(check: (usize, usize, usize), bounds: (f32, f32, f32)) -> bool {
    (check.0 as f32) < bounds.0 || (check.1 as f32) < bounds.1 || (check.2 as f32) < bounds.2
}

pub fn convert_vars_list_to_byondlist(
    parsed_map: &mut ParsedMapTranslationLayer,
    vars: &Option<Vec<(&str, Literal)>>,
) -> eyre::Result<ByondValue> {
    zone!("convert_vars_list_to_byondlist");
    if let Some(vars) = vars {
        let mut vars_list = ByondValue::new_list()?;
        for (key, literal) in vars {
            let value = convert_literal_to_byondvalue(parsed_map, key, literal)?;
            vars_list.write_list_index(ByondValue::new_str(*key)?, value)?;
        }
        Ok(vars_list)
    } else {
        Ok(ByondValue::null())
    }
}

/// This only hard errors when running into an internal BYOND error, such as bad proc, bad value, out of memory, etc
fn convert_literal_to_byondvalue(
    parsed_map: &mut ParsedMapTranslationLayer,
    key: &str,
    literal: &Literal,
) -> eyre::Result<ByondValue> {
    zone!("convert_literal_to_byondvalue");
    Ok(match literal {
        Literal::Number(n) => ByondValue::new_num(*n),
        Literal::String(s) => ByondValue::new_str(*s)?,
        Literal::Path(p) => _bapi_helper_text2path(p)?,
        Literal::File(f) => _bapi_helper_text2file(f)?,
        Literal::Null => ByondValue::null(),
        Literal::Fallback(s) => {
            parsed_map.add_warning(format!(
                "Parser failed to parse value for {:#?} and fellback to string: {s:#?}",
                key
            ))?;
            ByondValue::new_str(*s)?
        }
        Literal::List(l) => {
            zone!("convert_literal_to_byondvalue(list)");
            let mut list = ByondValue::new_list()?;

            for literal in l {
                match convert_literal_to_byondvalue(parsed_map, key, literal) {
                    Ok(item) => list.push_list(item)?,
                    Err(e) => {
                        parsed_map.add_warning(format!(
                            "Inside list inside {:#?}, failed to parse value: {e:#?}",
                            key
                        ))?;
                    }
                }
            }

            list
        }
        Literal::AssocList(map) => {
            zone!("convert_literal_to_byondvalue(assoc list)");
            let mut list = ByondValue::new_list()?;

            for (list_key, lit) in map.iter() {
                match convert_literal_to_byondvalue(parsed_map, key, lit) {
                    Ok(item) => list.write_list_index(ByondValue::new_str(*list_key)?, item)?,
                    Err(e) => {
                        parsed_map.add_warning(format!(
                            "Inside list inside {:#?}, failed to parse value: {e:#?}",
                            key
                        ))?;
                    }
                }
            }

            list
        }
    })
}
