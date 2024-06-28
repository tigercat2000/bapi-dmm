use byondapi::prelude::*;
use eyre::eyre;
use std::{cell::RefCell, fs::OpenOptions, io::Write, path::Path};

#[ignore = "Generates bindings in current directory"]
#[test]
fn generate_binds() {
    byondapi::generate_bindings(env!("CARGO_CRATE_NAME"));
}

fn write_log<T: AsRef<[u8]>>(x: T) {
    OpenOptions::new()
        .append(true)
        .create(true)
        .open("./rust_log.txt")
        .unwrap()
        .write_all(x.as_ref())
        .unwrap();
}

fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        write_log(format!("Panic {:#?}", info));
    }))
}

#[byondapi::bind]
/// Returns "10" if loaded correctly
pub fn _bapidmm_test_connection() {
    Ok(ByondValue::new_num(10f32))
}

#[ouroboros::self_referencing]
struct Map {
    map_data: String,
    #[borrows(map_data)]
    #[covariant]
    parsed_data: (dmm_lite::MapInfo, dmm_lite::MapData<'this>),
}

thread_local! {
    static PARSED_MAPS: RefCell<Vec<Map>> = const { RefCell::new(vec![]) };
}

#[byondapi::bind]
/// This is a dumb function: It will simply parse the file you tell it to
/// Any caching must be done in DM
pub fn _bapidmm_parse_map_blocking(dmm_file: ByondValue) {
    setup_panic_handler();

    let dmm_file = dmm_file.get_string()?;

    let path = Path::new(&dmm_file);
    if !path.is_file() {
        return Err(eyre!("Unable to find {dmm_file:#?} on disk"));
    }

    let string =
        std::fs::read_to_string(path).map_err(|e| eyre!("Failed to read {dmm_file:#?}: {e:#?}"))?;

    let map = MapTryBuilder {
        map_data: string,
        parsed_data_builder: |map_data: &String| dmm_lite::parse_map_multithreaded(map_data),
    }
    .try_build()
    .map_err(|e| eyre!("Error parsing {dmm_file:#?}: {e:#?}"))?;
    let mut new_parsed_map =
        ByondValue::builtin_new(ByondValue::new_str("/datum/bapi_parsed_map")?, &[])
            .map_err(|e| eyre!("Failed to create parsed map datum: {e:#?}"))?;

    new_parsed_map.write_var(
        "map_format",
        &ByondValue::new_str(if map.borrow_parsed_data().0.is_tgm {
            "tgm"
        } else {
            "dmm"
        })?,
    )?;

    find_metadata(&mut new_parsed_map, map.borrow_parsed_data())?;

    let index = PARSED_MAPS.with_borrow_mut(|f| {
        f.push(map);
        f.len() - 1
    });

    new_parsed_map.write_var("_internal_index", &ByondValue::new_num(index as f32))?;

    Ok(new_parsed_map)
}

#[byondapi::bind]
/// This function empties out the cached map data
pub fn _bapidmm_clear_map_data() {
    let _ = PARSED_MAPS.replace(vec![]);
    Ok(ByondValue::null())
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
