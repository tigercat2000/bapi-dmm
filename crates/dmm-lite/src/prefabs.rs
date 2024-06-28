use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use std::collections::HashMap;
use winnow::{
    ascii::{alpha1, dec_int, float, line_ending, multispace0, space0, space1},
    combinator::{alt, delimited, opt, peek, repeat, separated_pair, terminated},
    error::{ErrMode, StrContext},
    prelude::*,
    stream::Stream,
    token::{one_of, take, take_while},
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

pub fn parse_path<'s>(i: &mut &'s str) -> PResult<&'s str> {
    // ensure path starts with a `/` but don't actually eat it
    peek('/').parse_peek(*i)?;
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

type Prefab<'s> = (&'s str, Option<Vec<(&'s str, Literal<'s>)>>);
pub fn parse_prefab<'s>(i: &mut &'s str) -> PResult<Prefab<'s>> {
    alt((
        (parse_path, parse_var_list)
            .context(StrContext::Label("prefab with data"))
            .map(|(a, b)| (a, Some(b))),
        parse_path
            .context(StrContext::Label("prefab with only path"))
            .map(|a| (a, None)),
    ))
    .parse_next(i)
}

pub type PrefabLine<'s> = (&'s str, Vec<Prefab<'s>>);
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

pub type Prefabs<'s> = HashMap<&'s str, Vec<(&'s str, Option<Vec<(&'s str, Literal<'s>)>>)>>;
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

/// Post-processing: Separate each variable kv pair in the list
/// {var1="derp"; var2; var3=7} -> ["var1=\"derp\"", "var2", "var3=7"]
pub fn separate_var_list<'s>(i: &mut &'s str) -> PResult<Vec<&'s str>> {
    let mut count: usize = 0;
    let mut in_str = false;

    // Eat the starting "{"
    '{'.parse_next(i)?;

    let mut vars = vec![];
    let mut checkpoint = i.checkpoint();

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
            Ok(";") => {
                if !in_str {
                    // We hit the end of a var decl: we now need to push it into our vars lost
                    i.reset(&checkpoint);
                    let key_and_val = take(count).parse_next(i)?;
                    // Eat all the whitespace
                    vars.push(key_and_val.trim());
                    // Eat the semicolon
                    let _ = take(1usize).parse_next(i)?;
                    // Eat any space
                    let _ = multispace0.parse_next(i)?;
                    // Continue with a reset count and a new checkpoint
                    count = 0;
                    checkpoint = i.checkpoint();
                } else {
                    count += 1;
                }
            }
            Ok("}") => {
                // Only dip out if not in a string
                if !in_str {
                    // If we have something left in our buffer, we add it
                    if count > 0 {
                        i.reset(&checkpoint);
                        let key_and_val = take(count).parse_next(i)?.trim();
                        // Eat all the whitespace
                        vars.push(key_and_val);
                        // Eat the }
                        let _ = '}'.parse_next(i)?;
                    }
                    return Ok(vars);
                } else {
                    count += 1;
                }
            }
            // Everything else is included
            Ok(_) => {
                count += 1;
            }
        }
    }
}

/// Post-processing: Separate each variable into k and v
/// {var1="derp"; var2; var3=7} -> {"var1": Some("derp"), "var2": None, "var3": Some(7f32)}
pub fn parse_var_list<'s>(i: &mut &'s str) -> PResult<Vec<(&'s str, Literal<'s>)>> {
    let vars = separate_var_list(i)?;

    vars.into_iter()
        .map(|mut kv| (parse_var_list_key, parse_literal).parse_next(&mut kv))
        .collect()
}

pub fn parse_var_list_key<'s>(i: &mut &'s str) -> PResult<&'s str> {
    terminated(parse_identifier, " = ").parse_next(i)
}

pub fn parse_identifier<'s>(i: &mut &'s str) -> PResult<&'s str> {
    // Ensure it starts with a letter, not a number
    peek(one_of(('a'..='z', 'A'..='Z'))).parse_peek(*i)?;
    take_while(1.., ('a'..='z', 'A'..='Z', '0'..='9', '_')).parse_next(i)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal<'s> {
    Number(f32),
    String(&'s str),
    Path(&'s str),
    File(&'s str),
    Null,
    Fallback(&'s str),
    List(Vec<Literal<'s>>),
    AssocList(HashMap<&'s str, Literal<'s>>),
}

pub fn parse_literal<'s>(i: &mut &'s str) -> PResult<Literal<'s>> {
    match alt((
        parse_literal_number.map(Literal::Number),
        parse_literal_string.map(Literal::String),
        parse_literal_list,
        parse_path.map(Literal::Path),
        parse_literal_file.map(Literal::File),
        "null".map(|_| Literal::Null),
    ))
    .parse_next(i)
    {
        Err(ErrMode::Backtrack(_)) => Ok(Literal::Fallback(i)),
        Err(e) => Err(e),
        Ok(t) => Ok(t),
    }
}

pub fn parse_literal_number(i: &mut &str) -> PResult<f32> {
    alt((float, dec_int.map(|s: isize| s as f32))).parse_next(i)
}

pub fn parse_literal_string<'s>(i: &mut &'s str) -> PResult<&'s str> {
    // Must start with '"'
    '"'.parse_next(i)?;

    let mut count: usize = 0;
    let checkpoint = i.checkpoint();

    loop {
        match alt((r#"\""#, take(1usize))).parse_next(i) {
            Err(e) => return Err(e),
            // Ignore escaped quotes
            Ok(r#"\""#) => {
                count += 2;
            }
            // Switch str state to avoid ending early
            Ok("\"") => {
                i.reset(&checkpoint);
                let string_contents = take(count).parse_next(i)?;
                // Eat quote
                '"'.parse_next(i)?;
                return Ok(string_contents);
            }
            // Everything else is included
            Ok(_) => {
                count += 1;
            }
        }
    }
}

pub fn parse_literal_list<'s>(i: &mut &'s str) -> PResult<Literal<'s>> {
    // Must start with "list("
    "list(".parse_next(i)?;

    alt((
        // Lists are either associative
        repeat(
            1..,
            terminated(
                separated_pair(
                    alt((parse_literal_string, parse_identifier)),
                    delimited(space0, '=', space0),
                    parse_literal,
                ),
                delimited(space0, alt((',', ')')), space0),
            ),
        )
        .map(|pairs: HashMap<&str, Literal>| Literal::AssocList(pairs)),
        // Or not
        repeat(
            1..,
            terminated(parse_literal, delimited(space0, alt((',', ')')), space0)),
        )
        .map(|literals: Vec<Literal>| Literal::List(literals)),
    ))
    .parse_next(i)
}

pub fn parse_literal_file<'s>(i: &mut &'s str) -> PResult<&'s str> {
    // Must start with '
    '\''.parse_next(i)?;

    let mut count: usize = 0;
    let checkpoint = i.checkpoint();

    loop {
        match alt((r#"\'"#, take(1usize))).parse_next(i) {
            Err(e) => return Err(e),
            // Ignore escaped single quote
            Ok(r#"\'"#) => {
                count += 2;
            }
            // Switch str state to avoid ending early
            Ok("\'") => {
                i.reset(&checkpoint);
                let string_contents = take(count).parse_next(i)?;
                // Eat quote
                '\''.parse_next(i)?;
                return Ok(string_contents);
            }
            // Everything else is included
            Ok(_) => {
                count += 1;
            }
        }
    }
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
            Ok((
                "/turf/open/space/basic",
                Some(vec![("name", Literal::String("meow"))])
            ))
        );
        assert_eq!(
            parse_prefab.parse_next(&mut attack_prefab),
            Ok((
                "/turf/open/space/basic",
                Some(vec![("name", Literal::String(r#"meo\"w}"#))])
            ))
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
                    (
                        "/obj/structure/disposalpipe/segment",
                        Some(vec![("dir", Literal::Number(4.))])
                    ),
                    ("/obj/effect/turf_decal/tile/neutral", None),
                    ("/turf/open/floor/iron", None),
                    ("/area/station/hallway/primary/central", None)
                ]
            ))
        );
    }

    #[test]
    fn test_prefab_var_separation() {
        let mut variables_dmm = r#"{dir = 10; network = list("Nadzedha Wall Colony")}"#;
        let mut variables_tgm = r#"{
	dir = 10;
	network = list("Nadzedha Wall Colony")
	}"#;
        assert_eq!(
            separate_var_list.parse_next(&mut variables_dmm),
            Ok(vec!["dir = 10", "network = list(\"Nadzedha Wall Colony\")"])
        );
        assert_eq!(
            separate_var_list.parse_next(&mut variables_tgm),
            Ok(vec!["dir = 10", "network = list(\"Nadzedha Wall Colony\")"])
        );

        let mut cursed_test_case =
            r#"{dir = 10; network = list(a = "\";meower\"", b = "mro;wl", c = "me}o;w")}"#;

        assert_eq!(
            separate_var_list.parse_next(&mut cursed_test_case),
            Ok(vec![
                "dir = 10",
                r#"network = list(a = "\";meower\"", b = "mro;wl", c = "me}o;w")"#
            ])
        )
    }

    #[test]
    fn test_identifier() {
        let mut valid_identifier = "abc1 = ";
        let mut bad_identifier = "bc~ = ";

        assert_eq!(
            parse_var_list_key.parse_next(&mut valid_identifier),
            Ok("abc1")
        );

        if let Ok(r) = parse_var_list_key.parse_next(&mut bad_identifier) {
            panic!("Bad identifier produced {r:?}")
        }
    }

    #[test]
    fn test_parse_literal() {
        let mut string_literal = r#""me\"ow""#;
        assert_eq!(
            parse_literal.parse_next(&mut string_literal),
            Ok(Literal::String(r#"me\"ow"#))
        );

        let mut list_of_strings = r#"list("meow", "meow2")"#;
        assert_eq!(
            parse_literal.parse_next(&mut list_of_strings),
            Ok(Literal::List(vec![
                Literal::String("meow"),
                Literal::String("meow2")
            ]))
        );

        let mut assoc_list = r#"list("meow"="meow2")"#;
        let mut expected_hashmap = HashMap::new();
        expected_hashmap.insert("meow", Literal::String("meow2"));
        assert_eq!(
            parse_literal.parse_next(&mut assoc_list),
            Ok(Literal::AssocList(expected_hashmap))
        );

        let mut float = "1.4";
        let mut fake_float = "1";
        let mut scary_float = "1e3";
        assert_eq!(
            parse_literal.parse_next(&mut float),
            Ok(Literal::Number(1.4))
        );
        assert_eq!(
            parse_literal.parse_next(&mut fake_float),
            Ok(Literal::Number(1.))
        );
        assert_eq!(
            parse_literal.parse_next(&mut scary_float),
            Ok(Literal::Number(1e3))
        );

        let mut path = "/obj/item";
        let mut bad_path = "obj/item";
        assert_eq!(
            parse_literal.parse_next(&mut path),
            Ok(Literal::Path("/obj/item"))
        );
        assert_eq!(
            parse_literal.parse_next(&mut bad_path),
            Ok(Literal::Fallback("obj/item"))
        );

        let mut file = r#"'icons/meow/me\'ow.png'"#;
        assert_eq!(
            parse_literal.parse_next(&mut file),
            Ok(Literal::File(r#"icons/meow/me\'ow.png"#))
        );
    }

    #[test]
    fn test_parse_var_list_full() {
        let mut omega_list = r#"{icon = 'icons/\'obj/crate.dmi'; name = "\"funny\" girl"; req_access = list(1, 2); req_one_access = list("meow" = 2, aaaa = 4); pixel_x = -7; spawns = /obj/item/meower; haha = 4e4; death = null; invalid = gmddmf}"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("meow", Literal::Number(2.));
        expected_map.insert("aaaa", Literal::Number(4.));

        assert_eq!(
            parse_var_list.parse_next(&mut omega_list),
            Ok(vec![
                ("icon", Literal::File(r#"icons/\'obj/crate.dmi"#)),
                ("name", Literal::String(r#"\"funny\" girl"#)),
                (
                    "req_access",
                    Literal::List(vec![Literal::Number(1.), Literal::Number(2.),])
                ),
                ("req_one_access", Literal::AssocList(expected_map)),
                ("pixel_x", Literal::Number(-7.)),
                ("spawns", Literal::Path("/obj/item/meower")),
                ("haha", Literal::Number(4e4)),
                ("death", Literal::Null),
                ("invalid", Literal::Fallback("gmddmf"))
            ])
        )
    }
}
