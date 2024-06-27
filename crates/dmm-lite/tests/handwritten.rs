use dmm_lite::{detect_tgm, get_prefab_locations, parse_prefab_line};
use winnow::Parser;

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

    assert!(!detect_tgm(&mut meow.as_str()));
    assert!(detect_tgm(&mut meow_tgm.as_str()));
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
        parse_prefab_line.parse_next(&mut meow.as_str()),
        Ok((
            "aaa",
            vec![
                ("/turf/space", Some(r#"{name = "meow"}"#)),
                ("/area/space", None)
            ]
        ))
    );
    assert_eq!(
        parse_prefab_line.parse_next(&mut meow_tgm.as_str()),
        Ok((
            "aaa",
            vec![
                ("/turf/space", Some("{\n\tname = \"meow\"\n\t}")),
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
        let mut parse = &meow[loc..];
        assert!(parse_prefab_line.parse_next(&mut parse).is_ok())
    }

    let meow_tgm_locations = get_prefab_locations(&meow_tgm);
    for loc in meow_tgm_locations {
        let mut parse = &meow_tgm[loc..];
        assert!(parse_prefab_line.parse_next(&mut parse).is_ok())
    }
}
