use dmm_lite::{
    block::{get_block_locations, parse_block},
    parse_map_multithreaded,
    prefabs::{detect_tgm, get_prefab_locations, parse_prefab_line, Literal},
};
use winnow::{Located, Parser as _};

#[test]
fn test_tgm_detection() {
    let nadezhda = std::fs::read_to_string("./tests/maps/nadezhda.dmm").unwrap();
    let nadezhda_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let nadezhda_tgm: String = nadezhda_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    assert!(!detect_tgm(&nadezhda));
    assert!(detect_tgm(&nadezhda_tgm));
}

#[test]
fn test_prefab_detection() {
    let nadezhda = std::fs::read_to_string("./tests/maps/nadezhda.dmm").unwrap();
    let nadezhda_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let nadezhda_tgm: String = nadezhda_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    let nadezhda_location_count = get_prefab_locations(&nadezhda).len();
    let nadezhda_tgm_location_count = get_prefab_locations(&nadezhda_tgm).len();

    assert_eq!(nadezhda_location_count, nadezhda_tgm_location_count);
    assert_eq!(nadezhda_location_count, 14980);
}

#[test]
fn test_prefab_line() {
    let nadezhda = std::fs::read_to_string("./tests/maps/nadezhda.dmm").unwrap();
    #[allow(clippy::format_collect)] // I'm not figuring out fold for a test case
    let nadezhda: String = nadezhda
        .lines()
        .skip(12)
        .map(|l| format!("{}\n", l))
        .collect();
    let nadezhda_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let nadezhda_tgm: String = nadezhda_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(66)
        .take(13)
        .collect();

    println!("nadezhda {nadezhda_tgm:#?}");

    assert_eq!(
        parse_prefab_line.parse_next(&mut Located::new(nadezhda.as_str())),
        Ok((
            "aaN",
            vec![
                (
                    "/obj/machinery/embedded_controller/radio/airlock/airlock_controller",
                    Some(vec![
                        ("pixel_y", Literal::Number(24.)),
                        ("frequency", Literal::Number(1380.)),
                        ("id_tag", Literal::String("vasiliy_dokuchaev_shuttle1")),
                        (
                            "tag_exterior_door",
                            Literal::String("research_shuttle_outer_back")
                        ),
                        (
                            "tag_interior_door",
                            Literal::String("research_shuttle_inner_back")
                        ),
                        ("req_access", Literal::List(vec![Literal::Number(13.)])),
                        ("tag_airpump", Literal::String("research_shuttle_pump_back")),
                        (
                            "tag_chamber_sensor",
                            Literal::String("research_shuttle_sensor_back")
                        ),
                    ])
                ),
                ("/turf/simulated/floor/reinforced", None),
                ("/area/shuttle/vasiliy_shuttle_area", None)
            ]
        ))
    );
    assert_eq!(
        parse_prefab_line.parse_next(&mut Located::new(nadezhda_tgm.as_str())),
        Ok((
            "aaN",
            vec![
                (
                    "/obj/machinery/embedded_controller/radio/airlock/airlock_controller",
                    Some(vec![
                        ("pixel_y", Literal::Number(24.)),
                        ("frequency", Literal::Number(1380.)),
                        ("id_tag", Literal::String("vasiliy_dokuchaev_shuttle1")),
                        (
                            "tag_exterior_door",
                            Literal::String("research_shuttle_outer_back")
                        ),
                        (
                            "tag_interior_door",
                            Literal::String("research_shuttle_inner_back")
                        ),
                        ("req_access", Literal::List(vec![Literal::Number(13.)])),
                        ("tag_airpump", Literal::String("research_shuttle_pump_back")),
                        (
                            "tag_chamber_sensor",
                            Literal::String("research_shuttle_sensor_back")
                        ),
                    ])
                ),
                ("/turf/simulated/floor/reinforced", None),
                ("/area/shuttle/vasiliy_shuttle_area", None)
            ]
        ))
    );
}

#[test]
fn full_prefab_parse() {
    let nadezhda = std::fs::read_to_string("./tests/maps/nadezhda.dmm").unwrap();
    let nadezhda_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm").unwrap();

    let nadezhda_locations = get_prefab_locations(&nadezhda);
    for loc in nadezhda_locations {
        let mut parse = Located::new(&nadezhda[loc..]);
        parse_prefab_line
            .parse_next(&mut parse)
            .expect("Prefab didn't parse correctly");
    }

    let nadezhda_tgm_locations = get_prefab_locations(&nadezhda_tgm);
    for loc in nadezhda_tgm_locations {
        let mut parse = Located::new(&nadezhda_tgm[loc..]);
        parse_prefab_line
            .parse_next(&mut parse)
            .expect("Prefab didn't parse correctly");
    }
}

#[test]
fn test_block_detection() {
    let nadezhda = std::fs::read_to_string("./tests/maps/nadezhda.dmm").unwrap();
    let nadezhda_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm").unwrap();
    // tgm files sometimes have a header
    // //MAP CONVERTED BY dmm2tgm.py THIS HEADER COMMENT PREVENTS RECONVERSION, DO NOT REMOVE
    let nadezhda_tgm: String = nadezhda_tgm
        .lines()
        .map(|l| format!("{}\n", l))
        .skip(1)
        .collect();

    let nadezhda_location_count = get_block_locations(&nadezhda).len();
    assert_eq!(nadezhda_location_count, 3);
    let nadezhda_tgm_location_count = get_block_locations(&nadezhda_tgm).len();
    assert_eq!(nadezhda_tgm_location_count, 200 * 3);
}

#[test]
fn full_block_parse() {
    let nadezhda = std::fs::read_to_string("./tests/maps/nadezhda.dmm").unwrap();
    let nadezhda_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm").unwrap();

    let nadezhda_locations = get_block_locations(&nadezhda);
    for loc in nadezhda_locations {
        let mut parse = &nadezhda[loc..];
        let value = parse_block.parse_next(&mut parse);
        match value {
            Ok(_) => {}
            Err(e) => panic!("Test Failed at {parse:#?}: {:#?}", e),
        }
    }

    let nadezhda_tgm_locations = get_block_locations(&nadezhda_tgm);
    for loc in nadezhda_tgm_locations {
        let mut parse = &nadezhda_tgm[loc..];
        let value = parse_block.parse_next(&mut parse);
        match value {
            Ok(_) => {}
            Err(e) => panic!("Test Failed at {parse:#?}: {:#?}", e),
        }
    }
}

#[test]
fn full_parse() {
    let map = std::fs::read_to_string("./tests/maps/nadezhda.dmm").unwrap();
    let map_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm").unwrap();

    let (meta, (prefabs, blocks)) = parse_map_multithreaded(&map).unwrap();
    assert!(!meta.is_tgm);
    assert_eq!(prefabs.len(), 14980);
    assert_eq!(blocks.len(), 3);

    let (meta, (tgm_prefabs, tgm_blocks)) = parse_map_multithreaded(&map_tgm).unwrap();
    assert!(meta.is_tgm);
    assert_eq!(tgm_prefabs.len(), 14980);
    assert_eq!(tgm_blocks.len(), 200 * 3);
}
