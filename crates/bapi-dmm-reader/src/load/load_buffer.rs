#![allow(clippy::too_many_arguments)]
//! This is a variant of bapidmm loading where the maploading generates a list of commands,
//! to execute separately from doing expensive operations.

use byondapi::prelude::*;
use eyre::eyre;
use tracy_full::{frame, zone};

use crate::{
    _compat::setup_panic_handler,
    arena::ArenaMap,
    load::{
        command_buffer::{Command, CommandBuffer},
        helpers::{
            ParsedMapTranslationLayer, _bapi_helper_get_world_bounds,
            _bapi_helper_get_world_type_area, _bapi_helper_get_world_type_turf,
        },
    },
    PARSED_MAPS_ARENABASED,
};

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

    let internal_data = unsafe { PARSED_MAPS_ARENABASED.get_mut() }
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
}

/// if you generate usize::MAX command buffers in one round I can't help you I'm sorry
static mut COMMAND_BUFFER_ID: usize = 0;

fn generate_command_buffer<'a>(
    parsed_map: &mut ParsedMapTranslationLayer,
    internal_data: &'a mut ArenaMap<'a>,
    offset: (f32, f32, f32),
    crop_map: bool,
    no_changeturf: bool,
    // These MUST be f32 because they can be INFINITY
    lower_bounds: (f32, f32, f32),
    upper_bounds: (f32, f32, f32),
    place_on_top: bool,
    new_z: bool,
) -> eyre::Result<ByondValue> {
    // Safety: only ever called on main thread by BYOND
    unsafe { COMMAND_BUFFER_ID += 1 };
    zone!("generate_command_buffer");

    let (_metadata, (prefabs, blocks)) = &internal_data.parsed_data;
    let command_buffers = &mut internal_data.command_buffers;
    let resume_key = unsafe { COMMAND_BUFFER_ID };

    let mut our_command_buffer = CommandBuffer::default();

    let key_len = parsed_map.get_key_len()?;
    let parsed_bounds = parsed_map.get_parsed_bounds()?;
    let world_bounds = _bapi_helper_get_world_bounds()?;
    our_command_buffer.cached_turfs.world_bounds = world_bounds;

    // Expand map if necessary
    if !crop_map {
        let max_extent_offset = (
            offset.0 as usize + parsed_bounds.3 - 1,
            offset.1 as usize + parsed_bounds.4 - 1,
            offset.2 as usize + parsed_bounds.5 - 1,
        );
        if exceeds_upper_bounds(max_extent_offset, world_bounds) && !crop_map {
            parsed_map.expand_map(max_extent_offset, new_z, offset.2)?;
            our_command_buffer.cached_turfs.world_bounds = max_extent_offset;
        }
    }

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

                // Avoid generating OOB commands
                if exceeds_upper_bounds(exact_coord, world_bounds) && crop_map {
                    continue;
                }

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

                    our_command_buffer.cached_turfs.cache(exact_coord)?;

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
                        our_command_buffer.commands.push_back(Command::CreateArea {
                            loc: exact_coord,
                            prefab: prefab_area,
                            new_z,
                        });
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
                        our_command_buffer.commands.push_back(Command::CreateTurf {
                            loc: exact_coord,
                            prefab: prefab_turf,
                            no_changeturf: no_afterchange,
                            place_on_top,
                        })
                    }

                    // We reverse it again after doing the turf and area
                    for instance in prefab_list.rev() {
                        // We allow these but warn about them
                        if !instance.0.starts_with("/obj") && !instance.0.starts_with("/mob") {
                            if instance.0.starts_with("/turf") {
                                parsed_map.add_warning(
                                    format!(
                                        "Prefab {prefab_key:#?} had a secondary turf that we aren't going to deal with: {instance:#?}"
                                    ))?;
                                continue;
                            } else {
                                parsed_map.add_warning(
                                        format!(
                                            "Prefab {prefab_key:#?} has a strange element that we'll treat as a movable: {instance:#?}"
                                        ))?;
                            }
                        }
                        zone!("generating CreateAtom");
                        // Movables are easy
                        our_command_buffer.commands.push_back(Command::CreateAtom {
                            loc: exact_coord,
                            prefab: instance,
                        });
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

    #[cfg(feature = "dump")]
    let _ = std::fs::write(
        format!("data/mapdump_{}_{}", _metadata.name, resume_key),
        format!("{:#?}", &our_command_buffer),
    );
    command_buffers.insert(resume_key, our_command_buffer);

    Ok(ByondValue::new_num(resume_key as f32))
}

// Helpers
fn exceeds_upper_bounds(check: (usize, usize, usize), bounds: (usize, usize, usize)) -> bool {
    check.0 > bounds.0 || check.1 > bounds.1 || check.2 > bounds.2
}

fn exceeds_lower_bounds(check: (usize, usize, usize), bounds: (usize, usize, usize)) -> bool {
    check.0 < bounds.0 || check.1 < bounds.1 || check.2 < bounds.2
}

fn float_exceeds_upper_bounds(check: (usize, usize, usize), bounds: (f32, f32, f32)) -> bool {
    (check.0 as f32) > bounds.0 || (check.1 as f32) > bounds.1 || (check.2 as f32) > bounds.2
}

fn float_exceeds_lower_bounds(check: (usize, usize, usize), bounds: (f32, f32, f32)) -> bool {
    (check.0 as f32) < bounds.0 || (check.1 as f32) < bounds.1 || (check.2 as f32) < bounds.2
}

fn separate_turfs(mut s: &str, n: usize) -> impl Iterator<Item = &'_ str> {
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
