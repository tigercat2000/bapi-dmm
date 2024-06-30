#![allow(clippy::too_many_arguments)]

// dmm_suite compatibility
use byondapi::{
    global_call::call_global,
    map::{byond_locatexyz, ByondXYZ},
    prelude::*,
};
use dmm_lite::prefabs::Literal;
use eyre::eyre;

use crate::{ouroboros_impl_map::Map, PARSED_MAPS};

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
    let new_z = new_z.get_number()?;

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

        Ok(ByondValue::null())
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
    new_z: f32,
) -> eyre::Result<()> {
    let data = internal_data.borrow_parsed_data();

    let prefabs = &data.1 .0;
    let blocks = &data.1 .1;

    let key_len = parsed_map.read_number("key_len")?;

    let world_bounds = byondapi::global_call::call_global("_bapi_helper_get_world_bounds", &[])?;
    let world_bounds = (
        world_bounds.read_list_index(1.)?.get_number()? as usize,
        world_bounds.read_list_index(2.)?.get_number()? as usize,
        world_bounds.read_list_index(3.)?.get_number()? as usize,
    );

    for (top_left, block) in blocks {
        for (map_y_offset, line) in block.iter().enumerate() {
            let turfs = separate_turfs(line, key_len as usize);
            for (map_x_offset, prefab_key) in turfs.enumerate() {
                let relative_coord = (
                    top_left.0 + map_x_offset,
                    top_left.1 + map_y_offset,
                    top_left.2,
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
                let exact_coord = (
                    relative_coord.0 + offset.0 as usize,
                    relative_coord.1 + offset.1 as usize,
                    relative_coord.2 + offset.2 as usize,
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
                        call_global(
                            "_bapi_expand_map",
                            &[
                                ByondValue::new_num(exact_coord.0 as f32),
                                ByondValue::new_num(exact_coord.1 as f32),
                                ByondValue::new_num(exact_coord.2 as f32),
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
                    continue;
                }

                if let Some(prefab) = prefabs.get(prefab_key) {
                    for (path_text, vars) in prefab.iter().rev() {
                        // ignore the hard stuff
                        if path_text.starts_with("/area") || path_text.starts_with("/turf") {
                            continue;
                        }

                        let path = call_global(
                            "_bapi_helper_text2path",
                            &[ByondValue::new_str(*path_text)?],
                        )?;
                        let mut created_atom = ByondValue::builtin_new(path, &[turf])?;

                        if let Some(vars) = vars {
                            for (name, value) in vars {
                                let value =
                                    convert_literal_to_byondvalue(&mut parsed_map, name, value)?;
                                let result = created_atom.write_var(*name, &value);
                                if let Err(e) = result {
                                    parsed_map.call(
                                        "_bapi_add_warning",
                                        &[ByondValue::new_str(format!(
                                            "Prefab specified a bad variable {name:#?}: {e:#?}"
                                        ))?],
                                    )?;
                                }
                            }
                        }
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

/// This only hard errors when running into an internal BYOND error, such as bad proc, bad value, out of memory, etc
fn convert_literal_to_byondvalue(
    parsed_map: &mut ByondValue,
    key: &str,
    literal: &Literal,
) -> eyre::Result<ByondValue> {
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
        Literal::AssocList(_) => todo!(),
    })
}
