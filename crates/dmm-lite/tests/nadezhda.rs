use dmm_lite::{
    block::{get_block_locations, parse_block},
    prefabs::{detect_tgm, get_prefab_locations, parse_prefab_line},
};
use winnow::Parser as _;

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

    assert!(!detect_tgm(&mut nadezhda.as_str()));
    assert!(detect_tgm(&mut nadezhda_tgm.as_str()));
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
        parse_prefab_line.parse_next(&mut nadezhda.as_str()),
        Ok((
            "aaN",
            vec![
                (
                    "/obj/machinery/embedded_controller/radio/airlock/airlock_controller",
                    Some(
                        "{pixel_y = 24; frequency = 1380; id_tag = \"vasiliy_dokuchaev_shuttle1\"; tag_exterior_door = \"research_shuttle_outer_back\"; tag_interior_door = \"research_shuttle_inner_back\"; req_access = list(13); tag_airpump = \"research_shuttle_pump_back\"; tag_chamber_sensor = \"research_shuttle_sensor_back\"}"
                    )
                ),
                ("/turf/simulated/floor/reinforced", None),
                ("/area/shuttle/vasiliy_shuttle_area", None)
            ]
        ))
    );
    assert_eq!(
        parse_prefab_line.parse_next(&mut nadezhda_tgm.as_str()),
        Ok((
            "aaN",
            vec![
                (
                    "/obj/machinery/embedded_controller/radio/airlock/airlock_controller",
                    Some(
                        r#"{
	pixel_y = 24;
	frequency = 1380;
	id_tag = "vasiliy_dokuchaev_shuttle1";
	tag_exterior_door = "research_shuttle_outer_back";
	tag_interior_door = "research_shuttle_inner_back";
	req_access = list(13);
	tag_airpump = "research_shuttle_pump_back";
	tag_chamber_sensor = "research_shuttle_sensor_back"
	}"#
                    )
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
        let mut parse = &nadezhda[loc..];
        assert!(parse_prefab_line.parse_next(&mut parse).is_ok())
    }

    let nadezhda_tgm_locations = get_prefab_locations(&nadezhda_tgm);
    for loc in nadezhda_tgm_locations {
        let mut parse = &nadezhda_tgm[loc..];
        assert!(parse_prefab_line.parse_next(&mut parse).is_ok())
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
