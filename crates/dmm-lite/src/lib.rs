use winnow::{combinator::opt, PResult, Parser};

pub mod block;
pub mod prefabs;

pub type MapData<'s> = PResult<(prefabs::Prefabs<'s>, Vec<block::Block<'s>>)>;
pub fn parse_map_multithreaded(i: &str) -> MapData {
    let mut i = i;
    // just merk the dmm2tgm header
    let _ = opt(
        "//MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE",
    )
    .parse_next(&mut i)?;

    Ok((
        prefabs::multithreaded_parse_map_prefabs(i)?,
        block::multithreaded_parse_map_locations(i)?,
    ))
}
