use miette::{miette, LabeledSpan};
use winnow::{combinator::opt, error::ContextError, Located, Parser};

pub mod block;
pub mod prefabs;

#[derive(Debug)]
pub struct MapInfo {
    pub is_tgm: bool,
    pub name: String,
}

#[derive(Debug)]
pub struct LocatedError {
    pub key_offset: usize,
    pub main_offset: usize,
    pub underlying: ContextError,
}

impl LocatedError {
    pub fn debug_print(&self, input: &str) {
        let report = miette!(
            labels = vec![
                LabeledSpan::at_offset(self.key_offset, "Key causing cut"),
                LabeledSpan::at_offset(self.main_offset, "Last parsed")
            ],
            "{:#?}",
            self.underlying
        )
        .with_source_code(input.to_string());

        eprintln!("{:?}", report);
    }
}

pub type MapData<'s> = (prefabs::Prefabs<'s>, Vec<block::Block<'s>>);
pub fn parse_map_multithreaded(name: String, i: &str) -> Result<(MapInfo, MapData), LocatedError> {
    let mut i = Located::new(i);
    // just merk the dmm2tgm header
    let _ = opt(
        "//MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE",
    )
    .parse_next(&mut i)
    .map_err(|e| {
        if let Some(e) = e.into_inner() {
            LocatedError {
                key_offset: 0,
                main_offset: 0,
                underlying: e,
            }
        } else {
            panic!("Parser produced Incomplete")
        }
    })?;

    let is_tgm = prefabs::detect_tgm(&i);

    let prefab_map = prefabs::multithreaded_parse_map_prefabs(i)?;
    let block_list = block::multithreaded_parse_map_locations(i)?;

    Ok((MapInfo { name, is_tgm }, (prefab_map, block_list)))
}
