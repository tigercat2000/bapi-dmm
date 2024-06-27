use std::collections::HashMap;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use winnow::{
    ascii::{alpha1, line_ending, multispace0, space1},
    combinator::{alt, delimited, opt, repeat, separated_pair, terminated},
    error::StrContext,
    prelude::*,
    stream::Stream,
    token::{take, take_while},
};

pub fn parse_key<'s>(i: &mut &'s str) -> PResult<&'s str> {
    terminated(
        delimited((alt((line_ending, "")), '"'), alpha1, '"'),
        (delimited(space1, '=', space1), '('),
    )
    .parse_next(i)
}

pub fn detect_tgm(i: &mut &str) -> bool {
    (parse_key, line_ending).parse_next(i).is_ok()
}

pub fn valid_path_characters<'s>(i: &mut &'s str) -> PResult<&'s str> {
    take_while(1.., ('a'..='z', 'A'..='Z', '_', '0'..='9')).parse_next(i)
}

pub fn parse_path<'s>(i: &mut &'s str) -> PResult<&'s str> {
    take_while(1.., ('a'..='z', 'A'..='Z', '_', '0'..='9', '/')).parse_next(i)
}

pub fn parse_prefab_data<'s>(i: &mut &'s str) -> PResult<&'s str> {
    let mut count: usize = 0;
    let mut in_str = false;
    let main_checkpoint = i.checkpoint();

    // Enforce starting with `{`
    match '{'.parse_next(i) {
        Err(e) => return Err(e),
        // reset because we want '{' in our final output
        Ok(_) => i.reset(&main_checkpoint),
    }

    loop {
        match alt((r#"\""#, take(1usize))).parse_next(i) {
            Err(e) => return Err(e),
            // Ignore escaped quotes
            Ok(r#"\""#) => {
                count += 2;
            }
            // Switch str state to avoid ending early
            Ok("\"") => {
                count += 1;
                in_str = !in_str;
            }
            Ok("}") => {
                count += 1;
                // Only dip out if not in a string
                if !in_str {
                    i.reset(&main_checkpoint);
                    return take(count).parse_next(i);
                }
            }
            // Everything else is included
            Ok(_) => {
                count += 1;
            }
        }
    }
}

pub fn parse_prefab<'s>(i: &mut &'s str) -> PResult<(&'s str, Option<&'s str>)> {
    alt((
        (parse_path, parse_prefab_data)
            .context(StrContext::Label("prefab with data"))
            .map(|(a, b)| (a, Some(b))),
        parse_path
            .context(StrContext::Label("prefab with only path"))
            .map(|a| (a, None)),
    ))
    .parse_next(i)
}

pub type PrefabLine<'s> = (&'s str, Vec<(&'s str, Option<&'s str>)>);

pub fn parse_prefab_line<'s>(i: &mut &'s str) -> PResult<PrefabLine<'s>> {
    terminated(
        separated_pair(
            parse_key,
            multispace0,
            repeat(
                1..,
                terminated(terminated(parse_prefab, opt(',')), opt(line_ending)),
            ),
        ),
        ")",
    )
    .parse_next(i)
}

/// Used for multithreading: Uses a fast regex to get the starting location of every prefab key
pub fn get_prefab_locations(i: &str) -> Vec<usize> {
    let re = Regex::new(r#""([a-zA-Z]+)" = \("#).unwrap();

    let mut results = vec![];
    for offset in re
        .captures_iter(i)
        .filter_map(|c| c.get(0).map(|f| f.start()))
    {
        results.push(offset);
    }

    results
}

pub type Prefabs<'s> = HashMap<&'s str, Vec<(&'s str, Option<&'s str>)>>;
pub fn multithreaded_parse_map_prefabs(i: &str) -> PResult<Prefabs> {
    let locations = get_prefab_locations(i);

    locations
        .par_iter()
        .map(|loc| {
            let mut substring = &i[*loc..];
            parse_prefab_line(&mut substring)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key() {
        let mut key = r#""abc" = ("#;
        let mut newline_key = "\n\"abc\" = (";
        let mut winnewline_key = "\r\n\"abc\" = (";

        assert_eq!(parse_key.parse_next(&mut key), Ok("abc"));
        assert_eq!(parse_key.parse_next(&mut newline_key), Ok("abc"));
        assert_eq!(parse_key.parse_next(&mut winnewline_key), Ok("abc"));

        let mut badkey = r#" "abc" = ("#;
        parse_key
            .parse_next(&mut badkey)
            .expect_err("Bad key was parsed");
    }

    #[test]
    fn tgm_detection() {
        let mut dmm_key = r#""abc" = (/turf"#;
        let mut tgm_key = "\"abc\" = (\n/turf";

        assert!(!detect_tgm(&mut dmm_key));
        assert!(detect_tgm(&mut tgm_key));
    }

    #[test]
    fn test_parse_path() {
        let mut path = r#"/turf/open/space/basic"#;

        assert_eq!(
            parse_path.parse_next(&mut path),
            Ok("/turf/open/space/basic")
        );
    }

    #[test]
    fn test_prefab_data() {
        let prefab_data = r#"{name = "meow"}"#;
        let prefab_data_evil = r#"{name = "me\"ow}"}"#;

        assert_eq!(
            parse_prefab_data.parse_next(&mut prefab_data.to_owned().as_str()),
            Ok(prefab_data)
        );
        assert_eq!(
            parse_prefab_data.parse_next(&mut prefab_data_evil.to_owned().as_str()),
            Ok(prefab_data_evil)
        );
    }

    #[test]
    fn test_parse_prefab() {
        let mut prefab_path_only = r#"/turf/open/space/basic,"#;
        let mut prefab_with_vars = r#"/turf/open/space/basic{name = "meow"},"#;
        let mut attack_prefab = r#"/turf/open/space/basic{name = "meo\"w}"},"#;

        assert_eq!(
            parse_prefab.parse_next(&mut prefab_path_only),
            Ok(("/turf/open/space/basic", None))
        );
        assert_eq!(
            parse_prefab.parse_next(&mut prefab_with_vars),
            Ok(("/turf/open/space/basic", Some(r#"{name = "meow"}"#)))
        );
        assert_eq!(
            parse_prefab.parse_next(&mut attack_prefab),
            Ok(("/turf/open/space/basic", Some(r#"{name = "meo\"w}"}"#)))
        );
    }

    #[test]
    fn test_prefab_line() {
        let mut prefab_line = r#""aaa" = (/turf/open/space/basic,/area/space)"#;
        let mut complicated_prefab_line = r#""aar" = (/mob/living/basic/bot/cleanbot/autopatrol,/obj/structure/disposalpipe/segment{dir = 4},/obj/effect/turf_decal/tile/neutral,/turf/open/floor/iron,/area/station/hallway/primary/central)"#;

        assert_eq!(
            parse_prefab_line.parse_next(&mut prefab_line),
            Ok((
                "aaa",
                vec![("/turf/open/space/basic", None), ("/area/space", None),]
            ))
        );

        assert_eq!(
            parse_prefab_line.parse_next(&mut complicated_prefab_line),
            Ok((
                "aar",
                vec![
                    ("/mob/living/basic/bot/cleanbot/autopatrol", None),
                    ("/obj/structure/disposalpipe/segment", Some("{dir = 4}")),
                    ("/obj/effect/turf_decal/tile/neutral", None),
                    ("/turf/open/floor/iron", None),
                    ("/area/station/hallway/primary/central", None)
                ]
            ))
        );
    }
}
