#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;

// dmm_suite compatibility
use byondapi::{
    global_call::{call_global, call_global_id},
    map::{byond_locatexyz, ByondXYZ},
    prelude::*,
};
use dmm_lite::prefabs::Literal;
use eyre::eyre;
use tracy_full::{frame, zone};

use crate::{_compat::setup_panic_handler, ouroboros_impl_map::Map, PARSED_MAPS};

#[byondapi::bind]
pub fn _bapidmm_load_map(
    mut parsed_map: ByondValue,
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
    let id = parsed_map
        .read_number("_internal_index")
        .map_err(|e| eyre!("Unable to read /datum/bapi_parsed_map/_internal_index: {e:#?}"))?;
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

        parsed_map.write_var("loading", &ByondValue::new_num(1.))?;

        // Load map
        match load_map_impl(
            parsed_map,
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
                parsed_map.call(
                    "_bapi_add_warning",
                    &[ByondValue::new_str(format!(
                        "Loading failed due to error: {e:#}"
                    ))?],
                )?;
                return Err(e);
            }
        }

        parsed_map.write_var("loading", &ByondValue::new_num(0.))?;

        frame!();

        Ok(ByondValue::new_num(1.))
    })
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

/// WARNING ABOUT ERRORS:
/// - Errors in this procedure should only be completely unrecoverable.
/// - If you error out of this, the map will be left partially loaded.
fn load_map_impl(
    mut parsed_map: ByondValue,
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
    let data = internal_data.borrow_parsed_data();

    let prefabs = &data.1 .0;
    let blocks = &data.1 .1;

    let key_len = parsed_map.read_number("key_len")?;
    let parsed_bounds = parsed_map.read_var("parsed_bounds")?;
    let parsed_bounds = (
        parsed_bounds.read_list_index(1.)?.get_number()? as usize,
        parsed_bounds.read_list_index(2.)?.get_number()? as usize,
        parsed_bounds.read_list_index(3.)?.get_number()? as usize,
        parsed_bounds.read_list_index(4.)?.get_number()? as usize,
        parsed_bounds.read_list_index(5.)?.get_number()? as usize,
        parsed_bounds.read_list_index(6.)?.get_number()? as usize,
    );

    let world_bounds = byondapi::global_call::call_global("_bapi_helper_get_world_bounds", &[])?;
    let world_bounds = (
        world_bounds.read_list_index(1.)?.get_number()? as usize,
        world_bounds.read_list_index(2.)?.get_number()? as usize,
        world_bounds.read_list_index(3.)?.get_number()? as usize,
    );

    let mut created_areas: HashMap<&str, ByondValue> = HashMap::new();
    let mut path_map: HashMap<&str, ByondValue> = HashMap::new();

    let world_turf = call_global("_bapi_helper_get_world_type_turf", &[])?.get_string()?;
    let world_area = call_global("_bapi_helper_get_world_type_area", &[])?.get_string()?;

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
            parsed_map.call(
                "_bapi_add_warning",
                &[ByondValue::new_str(
                    "Z-level expansion occurred without no_changeturf set, this may cause problems when /turf/AfterChange is called, and therefore ChangeTurf will NOT be called"
                )?],
            )?;
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
                    parsed_map.call(
                        "_bapi_add_warning",
                        &[ByondValue::new_str(format!(
                            "Bad map coord (tries to spawn in negative space): {exact_coord:#?}"
                        ))?],
                    )?;
                    continue;
                }

                // Expand map if necessary
                if exceeds_upper_bounds(exact_coord, world_bounds) {
                    if crop_map {
                        continue;
                    } else {
                        parsed_map.call(
                            "_bapi_expand_map",
                            &[
                                ByondValue::new_num(exact_coord.0 as f32),
                                ByondValue::new_num(exact_coord.1 as f32),
                                ByondValue::new_num(exact_coord.2 as f32),
                                ByondValue::new_num(if new_z { 1. } else { 0. }),
                                ByondValue::new_num(offset.2),
                            ],
                        )?;
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
                    parsed_map.call(
                        "_bapi_add_warning",
                        &[ByondValue::new_str(format!(
                            "Failed to locate turf at: {exact_coord:#?}, skipping"
                        ))?],
                    )?;
                    continue;
                }

                if Some(prefab_key) == space_key && no_afterchange {
                    continue;
                }

                if let Some(prefab) = prefabs.get(prefab_key) {
                    // DMM prefab require that all prefab lists end with one /turf, and then one /area.
                    if prefab.len() < 2 {
                        parsed_map.call(
                            "_bapi_add_warning",
                            &[ByondValue::new_str(format!(
                                "Prefab {prefab_key:#?} is too short, violating requirement for /turf and /area!"
                            ))?],
                        )?;
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

                    zone!("prefab creation");

                    let mut prefab_list = prefab.iter().rev();
                    // Above check ensures that these cannot panic
                    let prefab_area = prefab_list.next().unwrap();
                    if !prefab_area.0.starts_with("/area") {
                        parsed_map.call(
                            "_bapi_add_warning",
                            &[ByondValue::new_str(format!(
                                "Prefab {prefab_key:#?} does not end in an area, instead ending in {prefab_area:#?}!"
                            ))?],
                        )?;
                        continue;
                    }
                    if !prefab_area.0.starts_with("/area/template_noop") {
                        let area = if let Some(area) = created_areas.get_mut(prefab_area.0) {
                            area
                        } else {
                            zone!("area creation");
                            let area = call_global(
                                "_bapi_create_or_get_area",
                                &[ByondValue::new_str(prefab_area.0)?],
                            )?;
                            created_areas.insert(prefab_area.0, area);
                            // This can't possibly fail, I hope
                            created_areas.get_mut(prefab_area.0).unwrap()
                        };

                        if !new_z {
                            zone!("_bapi_handle_area_contain");
                            call_global("_bapi_handle_area_contain", &[turf, *area])?;
                        }
                        zone!("_bapi_add_turf_to_area");
                        call_global_id(byond_string!("_bapi_add_turf_to_area"), &[*area, turf])?;
                    }

                    let prefab_turf = prefab_list.next().unwrap();
                    if !prefab_turf.0.starts_with("/turf") {
                        parsed_map.call(
                            "_bapi_add_warning",
                            &[ByondValue::new_str(format!(
                                "Prefab {prefab_key:#?} does not second-end in a turf, instead ending in {prefab_turf:#?}!"
                            ))?],
                        )?;
                        continue;
                    }
                    if !prefab_turf.0.starts_with("/turf/template_noop") {
                        zone!("turf creation");
                        create_turf(
                            &mut parsed_map,
                            &turf,
                            prefab_turf,
                            place_on_top,
                            no_afterchange,
                        )?;
                    }

                    // We reverse it again after doing the turf and area
                    for instance in prefab_list.rev() {
                        // We allow these but warn about them
                        if !instance.0.starts_with("/obj") && !instance.0.starts_with("/mob") {
                            parsed_map.call(
                                "_bapi_add_warning",
                                &[ByondValue::new_str(format!(
                                    "Prefab {prefab_key:#?} has a strange element that we'll treat as a movable: {instance:#?}"
                                ))?],
                            )?;
                        }
                        // Movables are easy
                        create_movable(&mut parsed_map, &mut path_map, &turf, instance)?;
                    }
                } else {
                    // Note: Cannot hard error or map will fail to finish loading
                    // This is necessarily just a warning
                    parsed_map.call(
                        "_bapi_add_warning",
                        &[ByondValue::new_str(format!(
                            "Invalid prefab key: {prefab_key:#?}"
                        ))?],
                    )?;
                }
            }
        }
    }

    let new_list = ByondValue::new_list()?;
    new_list.write_list(&[
        ByondValue::new_num(bounds.0 as f32),
        ByondValue::new_num(bounds.1 as f32),
        ByondValue::new_num(bounds.2 as f32),
        ByondValue::new_num(bounds.3 as f32),
        ByondValue::new_num(bounds.4 as f32),
        ByondValue::new_num(bounds.5 as f32),
    ])?;
    parsed_map.write_var("bounds", &new_list)?;

    Ok(())
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

fn create_movable<'s>(
    parsed_map: &mut ByondValue,
    path_cache: &mut HashMap<&'s str, ByondValue>,
    turf: &ByondValue,
    obj: &'s dmm_lite::prefabs::Prefab,
) -> eyre::Result<ByondValue> {
    zone!("movable creation");
    let (path_text, vars) = obj;
    let path = path_cache.entry(*path_text).or_insert_with(|| {
        zone!("creating path string");
        let text = ByondValue::new_str(*path_text).expect("Failed to allocate string");
        zone!("text2path");
        call_global("_bapi_helper_text2path", &[text]).expect("Failed to call text2path")
    });

    if vars.is_some() {
        let vars_list = convert_vars_list_to_byondlist(parsed_map, vars)?;
        zone!("setting up preloader");
        call_global("_bapi_setup_preloader", &[vars_list, *path])?;
    }

    zone!("byond_new");
    let instance = ByondValue::builtin_new(*path, &[*turf])?;
    zone!("apply preloader");
    call_global("_bapi_apply_preloader", &[instance])?;

    Ok(instance)
}

fn create_turf(
    parsed_map: &mut ByondValue,
    turf: &ByondValue,
    prefab_turf: &dmm_lite::prefabs::Prefab,
    place_on_top: bool,
    no_changeturf: bool,
) -> eyre::Result<ByondValue> {
    zone!("create_turf");
    let (path_text, vars) = prefab_turf;

    zone!("creating path string");
    let path_text = ByondValue::new_str(*path_text)?;
    let vars_list = convert_vars_list_to_byondlist(parsed_map, vars)?;
    let place_on_top = if place_on_top {
        ByondValue::new_num(1.)
    } else {
        ByondValue::new_num(0.)
    };
    let no_changeturf = if no_changeturf {
        ByondValue::new_num(1.)
    } else {
        ByondValue::new_num(0.)
    };

    zone!("_bapi_create_turf");
    call_global_id(
        byond_string!("_bapi_create_turf"),
        &[*turf, path_text, vars_list, place_on_top, no_changeturf],
    )
    .map_err(|e| eyre!("Failed to create turf: {e:#?}"))
}

fn convert_vars_list_to_byondlist(
    parsed_map: &mut ByondValue,
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
    parsed_map: &mut ByondValue,
    key: &str,
    literal: &Literal,
) -> eyre::Result<ByondValue> {
    zone!("convert_literal_to_byondvalue");
    Ok(match literal {
        Literal::Number(n) => ByondValue::new_num(*n),
        Literal::String(s) => ByondValue::new_str(*s)?,
        Literal::Path(p) => call_global("_bapi_helper_text2path", &[ByondValue::new_str(*p)?])?,
        Literal::File(f) => call_global("_bapi_helper_text2file", &[ByondValue::new_str(*f)?])?,
        Literal::Null => ByondValue::null(),
        Literal::Fallback(s) => {
            parsed_map.call(
                "_bapi_add_warning",
                &[ByondValue::new_str(format!(
                    "Parser failed to parse value for {:#?} and fellback to string: {s:#?}",
                    key
                ))?],
            )?;
            ByondValue::new_str(*s)?
        }
        Literal::List(l) => {
            zone!("convert_literal_to_byondvalue(list)");
            let mut list = ByondValue::new_list()?;

            for literal in l {
                match convert_literal_to_byondvalue(parsed_map, key, literal) {
                    Ok(item) => list.push_list(item)?,
                    Err(e) => {
                        parsed_map.call(
                            "_bapi_add_warning",
                            &[ByondValue::new_str(format!(
                                "Inside list inside {:#?}, failed to parse value: {e:#?}",
                                key
                            ))?],
                        )?;
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
                        parsed_map.call(
                            "_bapi_add_warning",
                            &[ByondValue::new_str(format!(
                                "Inside list inside {:#?}, failed to parse value: {e:#?}",
                                key
                            ))?],
                        )?;
                    }
                }
            }

            list
        }
    })
}
