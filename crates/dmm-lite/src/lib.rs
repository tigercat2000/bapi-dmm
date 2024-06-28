use winnow::{combinator::opt, stream::Stream, PResult, Parser};

pub mod block;
pub mod prefabs;

pub struct MapInfo {
    pub is_tgm: bool,
}

pub type MapData<'s> = (prefabs::Prefabs<'s>, Vec<block::Block<'s>>);
pub fn parse_map_multithreaded(i: &str) -> PResult<(MapInfo, MapData)> {
    let mut i = i;
    // just merk the dmm2tgm header
    let _ = opt(
        "//MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE",
    )
    .parse_next(&mut i)?;

    let checkpoint = i.checkpoint();
    let is_tgm = prefabs::detect_tgm(&mut i);
    i.reset(&checkpoint);

    let prefab_map = prefabs::multithreaded_parse_map_prefabs(i)?;
    let block_list = block::multithreaded_parse_map_locations(i)?;

    Ok((MapInfo { is_tgm }, (prefab_map, block_list)))
}
