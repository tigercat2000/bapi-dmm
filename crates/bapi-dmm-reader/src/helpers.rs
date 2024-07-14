//! This file is home to wrappers for BYOND-VM global procs we use to do our work.
//! This allows for strongly typed arguments, which I found out the hard way we really need.

use byondapi::{global_call::call_global, value::ByondValue};
use eyre::{Context, Result};
use tracy_full::zone;

/// Gets the current world.maxx, world.maxy, and world.maxz
pub fn _bapi_helper_get_world_bounds() -> Result<(usize, usize, usize)> {
    zone!("_bapi_helper_get_world_bounds");
    let world_bounds = call_global("_bapi_helper_get_world_bounds", &[])?;
    Ok((
        world_bounds.read_list_index(1.)?.get_number()? as usize,
        world_bounds.read_list_index(2.)?.get_number()? as usize,
        world_bounds.read_list_index(3.)?.get_number()? as usize,
    ))
}

/// Gets world.turf's typepath as a string
pub fn _bapi_helper_get_world_type_turf() -> Result<String> {
    zone!("_bapi_helper_get_world_type_turf");
    call_global("_bapi_helper_get_world_type_turf", &[])?
        .get_string()
        .context("Unable to get world.turf")
}

/// Gets world.area's typepath as a string
pub fn _bapi_helper_get_world_type_area() -> Result<String> {
    zone!("_bapi_helper_get_world_type_area");
    call_global("_bapi_helper_get_world_type_area", &[])?
        .get_string()
        .context("Unable to get world.area")
}

pub fn _bapi_create_or_get_area(path: &str) -> Result<ByondValue> {
    zone!("_bapi_create_or_get_area");
    call_global("_bapi_create_or_get_area", &[ByondValue::new_str(path)?])
        .context("Failed to create or get area")
}

pub fn _bapi_handle_area_contain(turf: ByondValue, area: ByondValue) -> Result<()> {
    zone!("_bapi_handle_area_contain");
    call_global("_bapi_handle_area_contain", &[turf, area])?;
    Ok(())
}

pub fn _bapi_add_turf_to_area(area: ByondValue, turf: ByondValue) -> Result<()> {
    zone!("_bapi_add_turf_to_area");
    call_global("_bapi_add_turf_to_area", &[area, turf])?;
    Ok(())
}

pub fn _bapi_helper_text2path(text: &str) -> Result<ByondValue> {
    zone!("_bapi_helper_text2path");
    call_global("_bapi_helper_text2path", &[ByondValue::new_str(text)?])
        .context("Failed to call text2path")
}

pub fn _bapi_helper_text2file(path: &str) -> Result<ByondValue> {
    zone!("_bapi_helper_text2file");
    call_global("_bapi_helper_text2file", &[ByondValue::new_str(path)?])
        .context("Failed to call text2file")
}

/// THE GODDAMN THING THAT MADE THIS FILE NECESSARY
pub fn _bapi_setup_preloader(vars_list: ByondValue, path: ByondValue) -> Result<()> {
    zone!("_bapi_setup_preloader");
    call_global("_bapi_setup_preloader", &[vars_list, path])?;
    Ok(())
}

pub fn _bapi_apply_preloader(instance: ByondValue) -> Result<()> {
    zone!("_bapi_apply_preloader");
    call_global("_bapi_apply_preloader", &[instance])?;
    Ok(())
}

pub fn _bapi_create_turf(
    turf: ByondValue,
    path_text: &str,
    vars_list: ByondValue,
    place_on_top: bool,
    no_changeturf: bool,
) -> Result<ByondValue> {
    zone!("_bapi_create_turf");
    call_global(
        "_bapi_create_turf",
        &[
            turf,
            ByondValue::new_str(path_text)?,
            vars_list,
            ByondValue::new_num(if place_on_top { 1. } else { 0. }),
            ByondValue::new_num(if no_changeturf { 1. } else { 0. }),
        ],
    )
    .context("Failed to call text2file")
}

pub fn _bapi_helper_tick_check() -> Result<bool> {
    let result = call_global("_bapi_helper_tick_check", &[])?;
    if result.is_true() {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Used to wrap calls and variable access on our /datum/bapi_parsed_map datum
pub struct ParsedMapTranslationLayer {
    pub parsed_map: ByondValue,
}

impl ParsedMapTranslationLayer {
    /// Add a warning for DM to see.
    pub fn add_warning<S: Into<Vec<u8>>>(&mut self, warning: S) -> Result<()> {
        self.parsed_map
            .call("_bapi_add_warning", &[ByondValue::new_str(warning)?])?;
        Ok(())
    }

    /// Expand the BYOND map to the new bounds. new_z and z_offset control whether or not it handles area contents for us.
    pub fn expand_map(
        &mut self,
        new_bounds: (usize, usize, usize),
        new_z: bool,
        z_offset: f32,
    ) -> Result<()> {
        self.parsed_map.call(
            "_bapi_expand_map",
            &[
                ByondValue::new_num(new_bounds.0 as f32),
                ByondValue::new_num(new_bounds.1 as f32),
                ByondValue::new_num(new_bounds.2 as f32),
                ByondValue::new_num(if new_z { 1. } else { 0. }),
                ByondValue::new_num(z_offset),
            ],
        )?;
        Ok(())
    }

    // Getters/Setters
    /// Get the _internal_index pointing into our ouroboros struct of parsed map data.
    pub fn get_internal_index(&self) -> Result<f32> {
        self.parsed_map
            .read_number("_internal_index")
            .context("Unable to read /datum/bapi_parsed_map/_internal_index")
    }

    /// Get the earlier-calculated key length without having to check again.
    pub fn get_key_len(&self) -> Result<f32> {
        self.parsed_map
            .read_number("key_len")
            .context("Failed to get key_len")
    }

    /// Get the parsed bounds of the map, the max extent if you will.
    pub fn get_parsed_bounds(&self) -> Result<(usize, usize, usize, usize, usize, usize)> {
        let parsed_bounds = self.parsed_map.read_var("parsed_bounds")?;
        Ok((
            parsed_bounds.read_list_index(1.)?.get_number()? as usize,
            parsed_bounds.read_list_index(2.)?.get_number()? as usize,
            parsed_bounds.read_list_index(3.)?.get_number()? as usize,
            parsed_bounds.read_list_index(4.)?.get_number()? as usize,
            parsed_bounds.read_list_index(5.)?.get_number()? as usize,
            parsed_bounds.read_list_index(6.)?.get_number()? as usize,
        ))
    }

    /// Set the loading var so byond knows if it can fuck with us or not. Probably irrelevant since we take control the whole time.
    pub fn set_loading(&mut self, loading: bool) -> Result<()> {
        self.parsed_map.write_var(
            "loading",
            &ByondValue::new_num(if loading { 1. } else { 0. }),
        )?;
        Ok(())
    }

    /// Set the bounds list with the actual extent of the map (that is to say, shit that isn't space turfs.)
    pub fn set_bounds(&mut self, bounds: (usize, usize, usize, usize, usize, usize)) -> Result<()> {
        let new_list = ByondValue::new_list()?;
        new_list.write_list(&[
            ByondValue::new_num(bounds.0 as f32),
            ByondValue::new_num(bounds.1 as f32),
            ByondValue::new_num(bounds.2 as f32),
            ByondValue::new_num(bounds.3 as f32),
            ByondValue::new_num(bounds.4 as f32),
            ByondValue::new_num(bounds.5 as f32),
        ])?;
        self.parsed_map.write_var("bounds", &new_list)?;
        Ok(())
    }
}
