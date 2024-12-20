use dmm_lite::{
    block::{get_block_locations, parse_block},
    parse_map_multithreaded,
    prefabs::{detect_tgm, get_prefab_locations, parse_prefab_line, Literal},
};
use winnow::{Located, Parser as _};

#[test]
fn test_tgm_detection() {
    let metastation = std::fs::read_to_string("./tests/maps/MetaStation.dmm").unwrap();
    let metastation_tgm = std::fs::read_to_string("./tests/maps/MetaStation-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let metastation_tgm: String = metastation_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    assert!(!detect_tgm(&metastation));
    assert!(detect_tgm(&metastation_tgm));
}

#[test]
fn test_prefab_detection() {
    let metastation = std::fs::read_to_string("./tests/maps/MetaStation.dmm").unwrap();
    let metastation_tgm = std::fs::read_to_string("./tests/maps/MetaStation-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let metastation_tgm: String = metastation_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    let metastation_location_count = get_prefab_locations(&metastation).len();
    let metastation_tgm_location_count = get_prefab_locations(&metastation_tgm).len();

    assert_eq!(metastation_location_count, metastation_tgm_location_count);
    assert_eq!(metastation_location_count, 8564);
}

#[test]
fn test_prefab_line() {
    let metastation = std::fs::read_to_string("./tests/maps/MetaStation.dmm").unwrap();
    #[allow(clippy::format_collect)] // I'm not figuring out fold for a test case
    let metastation: String = metastation
        .lines()
        .skip(11)
        .map(|l| format!("{}\n", l))
        .collect();
    let metastation_tgm = std::fs::read_to_string("./tests/maps/MetaStation-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let metastation_tgm: String = metastation_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(15)
        .take(10)
        .collect();

    assert_eq!(
        parse_prefab_line.parse_next(&mut Located::new(metastation.as_str())),
        Ok((
            "aal",
            vec![
                ("/obj/structure/cable", None),
                (
                    "/obj/machinery/atmospherics/pipe/smart/manifold4w/supply/hidden/layer4",
                    None
                ),
                (
                    "/obj/machinery/atmospherics/pipe/smart/manifold4w/scrubbers/hidden/layer2",
                    None
                ),
                ("/obj/structure/disposalpipe/segment", None),
                (
                    "/obj/effect/turf_decal/tile/neutral",
                    Some(vec![("dir", Literal::Number(4.))])
                ),
                ("/turf/open/floor/iron", None),
                ("/area/station/hallway/primary/port", None)
            ]
        ))
    );
    assert_eq!(
        parse_prefab_line.parse_next(&mut Located::new(metastation_tgm.as_str())),
        Ok((
            "aal",
            vec![
                ("/obj/structure/cable", None),
                (
                    "/obj/machinery/atmospherics/pipe/smart/manifold4w/supply/hidden/layer4",
                    None
                ),
                (
                    "/obj/machinery/atmospherics/pipe/smart/manifold4w/scrubbers/hidden/layer2",
                    None
                ),
                ("/obj/structure/disposalpipe/segment", None),
                (
                    "/obj/effect/turf_decal/tile/neutral",
                    Some(vec![("dir", Literal::Number(4.))])
                ),
                ("/turf/open/floor/iron", None),
                ("/area/station/hallway/primary/port", None)
            ]
        ))
    );
}

#[test]
fn full_prefab_parse() {
    let metastation = std::fs::read_to_string("./tests/maps/MetaStation.dmm").unwrap();
    let metastation_tgm = std::fs::read_to_string("./tests/maps/MetaStation-tgm.dmm").unwrap();

    let metastation_locations = get_prefab_locations(&metastation);
    for loc in metastation_locations {
        let mut parse = Located::new(&metastation[loc..]);
        parse_prefab_line
            .parse_next(&mut parse)
            .expect("Prefab didn't parse correctly");
    }

    let metastation_tgm_locations = get_prefab_locations(&metastation_tgm);
    for loc in metastation_tgm_locations {
        let mut parse = Located::new(&metastation_tgm[loc..]);
        parse_prefab_line
            .parse_next(&mut parse)
            .expect("Prefab didn't parse correctly");
    }
}

#[test]
fn test_block_detection() {
    let metastation = std::fs::read_to_string("./tests/maps/MetaStation.dmm").unwrap();
    let metastation_tgm = std::fs::read_to_string("./tests/maps/MetaStation-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let metastation_tgm: String = metastation_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    let metastation_location_count = get_block_locations(&metastation).len();
    assert_eq!(metastation_location_count, 1);
    let metastation_tgm_location_count = get_block_locations(&metastation_tgm).len();
    assert_eq!(metastation_tgm_location_count, 255);
}

#[test]
fn full_block_parse() {
    let metastation = std::fs::read_to_string("./tests/maps/MetaStation.dmm").unwrap();
    let metastation_tgm = std::fs::read_to_string("./tests/maps/MetaStation-tgm.dmm").unwrap();

    let metastation_locations = get_block_locations(&metastation);
    for loc in metastation_locations {
        let parse = &metastation[loc..];
        let value = parse_block.parse_next(&mut Located::new(parse));
        match value {
            Ok(_) => {}
            Err(e) => panic!("Test Failed at {parse:#?}: {:#?}", e),
        }
    }

    let metastation_tgm_locations = get_block_locations(&metastation_tgm);
    for loc in metastation_tgm_locations {
        let parse = &metastation_tgm[loc..];
        let value = parse_block.parse_next(&mut Located::new(parse));
        match value {
            Ok(_) => {}
            Err(e) => panic!("Test Failed at {parse:#?}: {:#?}", e),
        }
    }
}

#[test]
fn full_parse() {
    let map = std::fs::read_to_string("./tests/maps/MetaStation.dmm").unwrap();
    let map_tgm = std::fs::read_to_string("./tests/maps/MetaStation-tgm.dmm").unwrap();

    let (meta, (prefabs, blocks)) = parse_map_multithreaded("Meta".to_owned(), &map).unwrap();
    assert!(!meta.is_tgm);
    assert_eq!(prefabs.len(), 8564);
    assert_eq!(blocks.len(), 1);

    let (meta, (tgm_prefabs, tgm_blocks)) =
        parse_map_multithreaded("Meta".to_owned(), &map_tgm).unwrap();
    assert!(meta.is_tgm);
    assert_eq!(tgm_prefabs.len(), 8564);
    assert_eq!(tgm_blocks.len(), 255);

    // Testing a problem
    assert_eq!(
        tgm_prefabs.get("cWy"),
        Some(&vec![
            (
                "/obj/machinery/atmospherics/components/binary/pump",
                Some(vec![
                    ("dir", Literal::Number(8.)),
                    ("name", Literal::String("Distro to Waste"))
                ])
            ),
            (
                "/obj/effect/turf_decal/tile/yellow",
                Some(vec![("dir", Literal::Number(4.))])
            ),
            (
                "/turf/open/floor/iron/dark/corner",
                Some(vec![("dir", Literal::Number(1.))])
            ),
            ("/area/station/engineering/atmos/pumproom", None),
        ])
    );
}
