use byondapi::prelude::*;
use eyre::eyre;
use std::path::Path;

use crate::{_compat::setup_panic_handler, ouroboros_impl_map::MapTryBuilder, PARSED_MAPS};

const MAP_TGM: &str = "tgm";
const MAP_DMM: &str = "dmm";

#[byondapi::bind]
/// This is a dumb function: It will simply parse the file you tell it to
/// Any caching must be done in DM
pub fn _bapidmm_parse_map_blocking(dmm_file: ByondValue, mut map_datum: ByondValue) {
    setup_panic_handler();

    if !dmm_file.is_str() {
        return Err(eyre!("dmm_file was not a string: {dmm_file:#?}"));
    }

    let dmm_file_str = dmm_file.get_string()?;

    let path = Path::new(&dmm_file_str);
    if !path.is_file() {
        return Err(eyre!("Unable to find {dmm_file_str:#?} on disk"));
    }

    let string = std::fs::read_to_string(path)
        .map_err(|e| eyre!("Failed to read {dmm_file_str:#?}: {e:#?}"))?;

    let map = MapTryBuilder {
        map_data: string,
        parsed_data_builder: |map_data: &String| dmm_lite::parse_map_multithreaded(map_data),
    }
    .try_build()
    .map_err(|e| eyre!("Error parsing {dmm_file_str:#?}: {e:#?}"))?;

    map_datum.write_var("original_path", &dmm_file)?;

    map_datum.write_var(
        "map_format",
        &ByondValue::new_str(if map.borrow_parsed_data().0.is_tgm {
            MAP_TGM
        } else {
            MAP_DMM
        })?,
    )?;

    find_metadata(&mut map_datum, map.borrow_parsed_data())?;

    let index = PARSED_MAPS.with_borrow_mut(|f| {
        f.push(map);
        f.len() - 1
    });

    map_datum.write_var("_internal_index", &ByondValue::new_num(index as f32))?;

    Ok(ByondValue::new_num(1.0))
}

// Maploader bounds indices
/// The maploader index for the maps minimum x
const MAP_MINX: usize = 0;
/// The maploader index for the maps minimum y
const MAP_MINY: usize = 1;
/// The maploader index for the maps minimum z
const MAP_MINZ: usize = 2;
/// The maploader index for the maps maximum x
const MAP_MAXX: usize = 3;
/// The maploader index for the maps maximum y
const MAP_MAXY: usize = 4;
/// The maploader index for the maps maximum z
const MAP_MAXZ: usize = 5;

fn find_metadata(
    metadata: &mut ByondValue,
    map: &(dmm_lite::MapInfo, dmm_lite::MapData),
) -> eyre::Result<()> {
    let prefabs = &map.1 .0;
    let blocks = &map.1 .1;

    let key_len = prefabs.keys().next().map(|s| s.len()).unwrap_or(0);

    metadata.write_var("key_len", &ByondValue::new_num(key_len as f32))?;

    let line_len = blocks
        .iter()
        .next()
        .map(|(_coord, vec)| vec.iter().next().map(|s| s.len()).unwrap_or(0))
        .unwrap_or(0);

    metadata.write_var("line_len", &ByondValue::new_num(line_len as f32))?;

    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];

    for (coord, lines) in blocks.iter() {
        // So: maps are defined from top to bottom, left to right
        // This means that the minimum x and y will always be the minimum coord block we encounter
        bounds[MAP_MINX] = bounds[MAP_MINX].min(coord.0 as f32);
        bounds[MAP_MINY] = bounds[MAP_MINY].min(coord.1 as f32);
        // z-levels must have their own coord block, meaning min/max is simply what we encounter
        bounds[MAP_MINZ] = bounds[MAP_MINZ].min(coord.2 as f32);
        bounds[MAP_MAXZ] = bounds[MAP_MAXZ].max(coord.2 as f32);
        // Now the complicated part: max x, max y
        // maxx is coord x + line length (in tiles, so divided by key_len), minus one because the left edge is at (x)
        // Every z-level must be the same size, so we don't have to calculate line length per map
        bounds[MAP_MAXX] = bounds[MAP_MAXX].max(coord.0 as f32 + (line_len / key_len) as f32 - 1.0);
        // maxy is slightly more complicated
        // maxy is coord y + the number of entries in the lines vector (already in tiles), minus one because the top edge is at (y)
        bounds[MAP_MAXY] = bounds[MAP_MAXY].max(coord.1 as f32 + lines.len() as f32 - 1.0);
    }

    if bounds.iter().any(|f| f.is_infinite()) {
        metadata.write_var("parsed_bounds", &ByondValue::null())?;
        metadata.write_var("bounds", &ByondValue::null())?;
    } else {
        let list = ByondValue::new_list()?;
        list.write_list(&bounds.map(ByondValue::new_num))?;
        metadata.write_var("parsed_bounds", &list)?;
        metadata.write_var("bounds", &list)?;
    }

    Ok(())
}
