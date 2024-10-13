use dmm_lite::{
    block::{get_block_locations, parse_block},
    parse_map_multithreaded,
    prefabs::{detect_tgm, get_prefab_locations, parse_prefab_line, Literal},
};
use winnow::{Located, Parser};

#[test]
fn test_tgm_detection() {
    let meow = std::fs::read_to_string("./tests/maps/handwritten.dmm").unwrap();
    let meow_tgm = std::fs::read_to_string("./tests/maps/handwritten-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let meow_tgm: String = meow_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    assert!(!detect_tgm(&meow));
    assert!(detect_tgm(&meow_tgm));
}

#[test]
fn test_prefab_detection() {
    let meow = std::fs::read_to_string("./tests/maps/handwritten.dmm").unwrap();
    let meow_tgm = std::fs::read_to_string("./tests/maps/handwritten-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let meow_tgm: String = meow_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    let meow_location_count = get_prefab_locations(&meow).len();
    let meow_tgm_location_count = get_prefab_locations(&meow_tgm).len();

    assert_eq!(meow_location_count, meow_tgm_location_count);
    assert_eq!(meow_location_count, 3);
}

#[test]
fn test_prefab_line() {
    let meow = std::fs::read_to_string("./tests/maps/handwritten.dmm").unwrap();
    let meow_tgm = std::fs::read_to_string("./tests/maps/handwritten-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let meow_tgm: String = meow_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    assert_eq!(
        parse_prefab_line.parse_next(&mut Located::new(meow.as_str())),
        Ok((
            "aaa",
            vec![
                ("/turf/space", Some(vec![("name", Literal::String("meow"))])),
                ("/area/space", None)
            ]
        ))
    );
    assert_eq!(
        parse_prefab_line.parse_next(&mut Located::new(meow_tgm.as_str())),
        Ok((
            "aaa",
            vec![
                ("/turf/space", Some(vec![("name", Literal::String("meow"))])),
                ("/area/space", None)
            ]
        ))
    );
}

#[test]
fn full_prefab_parse() {
    let meow = std::fs::read_to_string("./tests/maps/handwritten.dmm").unwrap();
    let meow_tgm = std::fs::read_to_string("./tests/maps/handwritten-tgm.dmm").unwrap();

    let meow_locations = get_prefab_locations(&meow);
    for loc in meow_locations {
        let mut parse = Located::new(&meow[loc..]);
        parse_prefab_line
            .parse_next(&mut parse)
            .expect("Prefab didn't parse correctly");
    }

    let meow_tgm_locations = get_prefab_locations(&meow_tgm);
    for loc in meow_tgm_locations {
        let mut parse = Located::new(&meow_tgm[loc..]);
        parse_prefab_line
            .parse_next(&mut parse)
            .expect("Prefab didn't parse correctly");
    }
}

#[test]
fn test_block_detection() {
    let meow = std::fs::read_to_string("./tests/maps/handwritten.dmm").unwrap();
    let meow_tgm = std::fs::read_to_string("./tests/maps/handwritten-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let meow_tgm: String = meow_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    let meow_location_count = get_block_locations(&meow).len();
    assert_eq!(meow_location_count, 1);
    let meow_tgm_location_count = get_block_locations(&meow_tgm).len();
    assert_eq!(meow_tgm_location_count, 3);
}

#[test]
fn test_single_block() {
    let meow = std::fs::read_to_string("./tests/maps/handwritten.dmm").unwrap();
    let meow: String = meow.lines().map(|l| format!("{}\n", l)).skip(4).collect();
    let meow_tgm = std::fs::read_to_string("./tests/maps/handwritten-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let meow_tgm: String = meow_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(13)
        .collect();

    assert_eq!(
        parse_block.parse_next(&mut Located::new(&meow)),
        Ok(((1, 1, 1), vec!["aaaaabaac", "aaaaabaac", "aaaaabaac"]))
    );
    assert_eq!(
        parse_block.parse_next(&mut Located::new(&meow_tgm)),
        Ok(((1, 1, 1), vec!["aaa"]))
    );
}

#[test]
fn full_block_parse() {
    let meow = std::fs::read_to_string("./tests/maps/handwritten.dmm").unwrap();
    let meow_tgm = std::fs::read_to_string("./tests/maps/handwritten-tgm.dmm").unwrap();

    let meow_locations = get_block_locations(&meow);
    for loc in meow_locations {
        let parse = &meow[loc..];
        let value = parse_block.parse_next(&mut Located::new(parse));
        match value {
            Ok(_) => {}
            Err(e) => panic!("Test Failed at {parse:#?}: {:#?}", e),
        }
    }

    let meow_tgm_locations = get_block_locations(&meow_tgm);
    for loc in meow_tgm_locations {
        let parse = &meow_tgm[loc..];
        let value = parse_block.parse_next(&mut Located::new(parse));
        match value {
            Ok(_) => {}
            Err(e) => panic!("Test Failed at {parse:#?}: {:#?}", e),
        }
    }
}

#[test]
fn full_parse() {
    let meow = std::fs::read_to_string("./tests/maps/handwritten.dmm").unwrap();
    let meow_tgm = std::fs::read_to_string("./tests/maps/handwritten-tgm.dmm").unwrap();

    let (meta, (prefabs, blocks)) = parse_map_multithreaded("Meow".to_owned(), &meow).unwrap();
    assert!(!meta.is_tgm);
    assert_eq!(prefabs.len(), 3);
    assert_eq!(blocks.len(), 1);

    let (meta, (tgm_prefabs, tgm_blocks)) =
        parse_map_multithreaded("Meow".to_owned(), &meow_tgm).unwrap();
    assert!(meta.is_tgm);
    assert_eq!(tgm_prefabs.len(), 3);
    assert_eq!(tgm_blocks.len(), 3);
}
