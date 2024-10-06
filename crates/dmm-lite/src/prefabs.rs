use miette::{miette, LabeledSpan, Severity};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use std::collections::HashMap;
use winnow::{
    ascii::{
        alpha0, alpha1, alphanumeric0, dec_int, float, line_ending, multispace0, space0, space1,
    },
    combinator::{
        alt, cut_err, delimited, fail, opt, peek, preceded, repeat, separated_pair, terminated,
    },
    error::{ErrMode, StrContext},
    prelude::*,
    stream::{Location, Offset, Stream},
    token::{one_of, take, take_while},
    Located,
};

use crate::LocatedError;

pub fn parse_key<'s>(i: &mut Located<&'s str>) -> PResult<&'s str> {
    terminated(
        delimited((alt((line_ending, "")), '"'), alpha1, '"'),
        (delimited(space1, '=', space1), '('),
    )
    .parse_next(i)
}

pub fn detect_tgm(i: &str) -> bool {
    (parse_key, line_ending)
        .parse_next(&mut Located::new(i))
        .is_ok()
}

pub fn parse_path<'s>(i: &mut Located<&'s str>) -> PResult<&'s str> {
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

pub type Prefab<'s> = (&'s str, Option<Vec<(&'s str, Literal<'s>)>>);
pub fn parse_prefab<'s>(i: &mut Located<&'s str>) -> PResult<Prefab<'s>> {
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
pub fn parse_prefab_line<'s>(i: &mut Located<&'s str>) -> PResult<PrefabLine<'s>> {
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
pub fn multithreaded_parse_map_prefabs(i: Located<&str>) -> Result<Prefabs, LocatedError> {
    let locations = get_prefab_locations(&i);

    locations
        .par_iter()
        .map(|loc| {
            let mut substring = Located::new(&i[*loc..]);
            parse_prefab_line(&mut substring).map_err(|e| {
                if let Some(e) = e.into_inner() {
                    LocatedError {
                        key_offset: i.location() + *loc,
                        main_offset: substring.location() + i.location() + *loc,
                        underlying: e,
                    }
                } else {
                    panic!("Parser produced Incomplete")
                }
            })
        })
        .collect()
}

/// Post-processing: Separate each variable kv pair in the list
/// {var1="derp"; var2; var3=7} -> ["var1=\"derp\"", "var2", "var3=7"]
pub fn separate_var_list<'s>(i: &mut Located<&'s str>) -> PResult<Vec<Located<&'s str>>> {
    let mut count: usize = 0;
    let mut in_str = false;

    // Eat the starting "{"
    '{'.context(StrContext::Expected(
        winnow::error::StrContextValue::Description("var list opener"),
    ))
    .parse_next(i)?;

    // From this point forward, we are committed until we find a matching `}`.

    let mut vars = vec![];
    let mut checkpoint = i.checkpoint();
    let first_checkpoint = i.checkpoint();
    let mut last_quote = 0;

    loop {
        match cut_err(
            alt((
                r#"\""#,
                take(1usize),
                fail.context(StrContext::Label("var list"))
                    .context(StrContext::Expected(
                        winnow::error::StrContextValue::StringLiteral(r#"\""#),
                    ))
                    .context(StrContext::Expected(
                        winnow::error::StrContextValue::Description("take(1)"),
                    )),
            ))
            .context(StrContext::Label("var list")),
        )
        .parse_next(i)
        {
            Err(e) => return Err(e),
            // Ignore escaped quotes
            Ok(r#"\""#) => {
                count += 2;
            }
            // Switch str state to avoid ending early
            Ok("\"") => {
                count += 1;
                in_str = !in_str;
                last_quote = i.offset_from(&first_checkpoint) - 1;
            }
            Ok(";") => {
                if !in_str {
                    // We hit the end of a var decl: we now need to push it into our vars lost
                    i.reset(&checkpoint);
                    let key_and_val = take(count).parse_next(i)?;
                    // Eat all the whitespace
                    vars.push(Located::new(key_and_val.trim()));
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
            Ok("\n") => {
                count += 1;
                if in_str {
                    let report = miette!(
                        severity = Severity::Warning,
                        labels = vec![LabeledSpan::at_offset(
                            last_quote,
                            "Start of unterminated string"
                        )],
                        "WARNING: Unterminated string literal terminated by line break"
                    );
                    i.reset(&first_checkpoint);
                    let report = report.with_source_code(i.to_string());

                    eprintln!("{:?}", report);

                    // To recover, we pretend we hit a `;`.
                    i.reset(&checkpoint);
                    // Just drop the var.
                    let _ = take(count).parse_next(i)?;
                    // Eat spaces.
                    let _ = multispace0.parse_next(i)?;
                    // Continue with a reset count and a new checkpoint.
                    count = 0;
                    checkpoint = i.checkpoint();

                    in_str = false;
                }
            }
            Ok("}") => {
                // Only dip out if not in a string
                if !in_str {
                    // If we have something left in our buffer, we add it
                    if count > 0 {
                        i.reset(&checkpoint);
                        let key_and_val = Located::new(take(count).parse_next(i)?.trim());
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
pub fn parse_var_list<'s>(i: &mut Located<&'s str>) -> PResult<Vec<(&'s str, Literal<'s>)>> {
    let vars = separate_var_list(i)?;

    vars.into_iter()
        .map(|mut kv| (parse_var_list_key, parse_literal).parse_next(&mut kv))
        .collect()
}

pub fn parse_var_list_key<'s>(i: &mut Located<&'s str>) -> PResult<&'s str> {
    terminated(parse_identifier, " = ").parse_next(i)
}

pub fn parse_identifier<'s>(i: &mut Located<&'s str>) -> PResult<&'s str> {
    // Ensure it starts with a letter or underscore, not a number
    peek(one_of(('a'..='z', 'A'..='Z', '_'))).parse_peek(*i)?;
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
    AssocList(Vec<(Literal<'s>, Literal<'s>)>),
}

pub fn parse_literal<'s>(i: &mut Located<&'s str>) -> PResult<Literal<'s>> {
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

pub fn parse_literal_number(i: &mut Located<&str>) -> PResult<f32> {
    alt((float, dec_int.map(|s: isize| s as f32))).parse_next(i)
}

pub fn parse_literal_string<'s>(i: &mut Located<&'s str>) -> PResult<&'s str> {
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

pub fn parse_bare_list_key<'s>(i: &mut Located<&'s str>) -> PResult<Literal<'s>> {
    terminated(preceded(peek(alpha0), alphanumeric0), peek(alt((' ', '='))))
        .map(Literal::Fallback)
        .parse_next(i)
}

pub fn parse_literal_list<'s>(i: &mut Located<&'s str>) -> PResult<Literal<'s>> {
    // Must start with "list("
    "list(".parse_next(i)?;

    // Special case: Empty lists
    if **i == ")" {
        return Ok(Literal::List(vec![]));
    }

    alt((
        // Lists are either associative
        repeat(
            1..,
            terminated(
                separated_pair(
                    alt((parse_bare_list_key, parse_literal)),
                    delimited(space0, '=', space0),
                    parse_literal,
                ),
                delimited(space0, alt((',', ')')), space0),
            ),
        )
        .map(|pairs: Vec<(Literal, Literal)>| Literal::AssocList(pairs)),
        // Or not
        repeat(
            1..,
            terminated(parse_literal, delimited(space0, alt((',', ')')), space0)),
        )
        .map(|literals: Vec<Literal>| Literal::List(literals)),
    ))
    .parse_next(i)
}

pub fn parse_literal_file<'s>(i: &mut Located<&'s str>) -> PResult<&'s str> {
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
        let mut key = Located::new(r#""abc" = ("#);
        let mut newline_key = Located::new("\n\"abc\" = (");
        let mut winnewline_key = Located::new("\r\n\"abc\" = (");

        assert_eq!(parse_key.parse_next(&mut key), Ok("abc"));
        assert_eq!(parse_key.parse_next(&mut newline_key), Ok("abc"));
        assert_eq!(parse_key.parse_next(&mut winnewline_key), Ok("abc"));

        let mut badkey = Located::new(r#" "abc" = ("#);
        parse_key
            .parse_next(&mut badkey)
            .expect_err("Bad key was parsed");
    }

    #[test]
    fn tgm_detection() {
        let dmm_key = r#""abc" = (/turf"#;
        let tgm_key = "\"abc\" = (\n/turf";

        assert!(!detect_tgm(dmm_key));
        assert!(detect_tgm(tgm_key));
    }

    #[test]
    fn test_parse_path() {
        let mut path = Located::new(r#"/turf/open/space/basic"#);

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
        let mut prefab_path_only = Located::new(r#"/turf/open/space/basic,"#);
        let mut prefab_with_vars = Located::new(r#"/turf/open/space/basic{name = "meow"},"#);
        let mut attack_prefab = Located::new(r#"/turf/open/space/basic{name = "meo\"w}"},"#);

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
        let mut prefab_line = Located::new(r#""aaa" = (/turf/open/space/basic,/area/space)"#);
        let mut complicated_prefab_line = Located::new(
            r#""aar" = (/mob/living/basic/bot/cleanbot/autopatrol,/obj/structure/disposalpipe/segment{dir = 4},/obj/effect/turf_decal/tile/neutral,/turf/open/floor/iron,/area/station/hallway/primary/central)"#,
        );

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
        let mut variables_dmm =
            Located::new(r#"{dir = 10; network = list("Nadzedha Wall Colony")}"#);
        let mut variables_tgm = Located::new(
            r#"{
    dir = 10;
    network = list("Nadzedha Wall Colony")
    }"#,
        );
        assert_eq!(
            separate_var_list
                .parse_next(&mut variables_dmm)
                .map(|s| s.iter().map(|s| **s).collect::<Vec<_>>()),
            Ok(vec!["dir = 10", "network = list(\"Nadzedha Wall Colony\")"])
        );
        assert_eq!(
            separate_var_list
                .parse_next(&mut variables_tgm)
                .map(|s| s.iter().map(|s| **s).collect::<Vec<_>>()),
            Ok(vec!["dir = 10", "network = list(\"Nadzedha Wall Colony\")"])
        );

        let mut cursed_test_case = Located::new(
            r#"{dir = 10; network = list(a = "\";meower\"", b = "mro;wl", c = "me}o;w")}"#,
        );

        assert_eq!(
            separate_var_list
                .parse_next(&mut cursed_test_case)
                .map(|s| s.iter().map(|s| **s).collect::<Vec<_>>()),
            Ok(vec![
                "dir = 10",
                r#"network = list(a = "\";meower\"", b = "mro;wl", c = "me}o;w")"#
            ])
        )
    }

    #[test]
    fn test_identifier() {
        let mut valid_identifier = Located::new("abc1 = ");
        let mut bad_identifier = Located::new("bc~ = ");

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
        let mut string_literal = Located::new(r#""me\"ow""#);
        assert_eq!(
            parse_literal.parse_next(&mut string_literal),
            Ok(Literal::String(r#"me\"ow"#))
        );

        let mut list_of_strings = Located::new(r#"list("meow", "meow2")"#);
        assert_eq!(
            parse_literal.parse_next(&mut list_of_strings),
            Ok(Literal::List(vec![
                Literal::String("meow"),
                Literal::String("meow2")
            ]))
        );

        let mut assoc_list = Located::new(r#"list("meow"="meow2")"#);
        assert_eq!(
            parse_literal.parse_next(&mut assoc_list),
            Ok(Literal::AssocList(vec![(
                Literal::String("meow"),
                Literal::String("meow2")
            )]))
        );

        let mut float = Located::new("1.4");
        let mut fake_float = Located::new("1");
        let mut scary_float = Located::new("1e3");
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

        let mut path = Located::new("/obj/item");
        let mut bad_path = Located::new("obj/item");
        assert_eq!(
            parse_literal.parse_next(&mut path),
            Ok(Literal::Path("/obj/item"))
        );
        assert_eq!(
            parse_literal.parse_next(&mut bad_path),
            Ok(Literal::Fallback("obj/item"))
        );

        let mut file = Located::new(r#"'icons/meow/me\'ow.png'"#);
        assert_eq!(
            parse_literal.parse_next(&mut file),
            Ok(Literal::File(r#"icons/meow/me\'ow.png"#))
        );
    }

    #[test]
    fn test_parse_bare_list_key() {
        let mut evil_key = Located::new(r#"aaa = 2"#);
        assert_eq!(
            parse_bare_list_key.parse_next(&mut evil_key),
            Ok(Literal::Fallback("aaa"))
        );
        assert_eq!(*evil_key, " = 2");

        let mut evil_list = Located::new("list(aaa = 2)");
        assert_eq!(
            parse_literal_list.parse_next(&mut evil_list),
            Ok(Literal::AssocList(vec![(
                Literal::Fallback("aaa"),
                Literal::Number(2.)
            )]))
        )
    }

    #[test]
    fn test_parse_var_list_full() {
        let mut omega_list = Located::new(
            r#"{icon = 'icons/\'obj/crate.dmi'; name = "\"funny\" girl"; req_access = list(1, 2); req_one_access = list("meow" = 2, aaaa = 4); pixel_x = -7; spawns = /obj/item/meower; haha = 4e4; death = null; invalid = gmddmf}"#,
        );

        assert_eq!(
            parse_var_list.parse_next(&mut omega_list),
            Ok(vec![
                ("icon", Literal::File(r#"icons/\'obj/crate.dmi"#)),
                ("name", Literal::String(r#"\"funny\" girl"#)),
                (
                    "req_access",
                    Literal::List(vec![Literal::Number(1.), Literal::Number(2.),])
                ),
                (
                    "req_one_access",
                    Literal::AssocList(vec![
                        (Literal::String("meow"), Literal::Number(2.)),
                        (Literal::Fallback("aaaa"), Literal::Number(4.)),
                    ])
                ),
                ("pixel_x", Literal::Number(-7.)),
                ("spawns", Literal::Path("/obj/item/meower")),
                ("haha", Literal::Number(4e4)),
                ("death", Literal::Null),
                ("invalid", Literal::Fallback("gmddmf"))
            ])
        )
    }

    #[test]
    fn test_weird_error_from_virgo() {
        let mut list = Located::new(
            r#""ii" = (
/obj/machinery/vending/engivend{
	products = list(/obj/item/device/geiger = 4, /obj/item/clothing/glasses/meson = 2);
	req_access = list(301);
	req_log_access = 301
	},
/turf/simulated/floor/tiled/techfloor/grid,
/area/talon_v2/engineering/star_store)"#,
        );

        let (key, prefabs) = parse_prefab_line(&mut list).unwrap();
        assert_eq!(key, "ii");
        assert_eq!(
            prefabs,
            vec![
                (
                    "/obj/machinery/vending/engivend",
                    Some(vec![
                        (
                            "products",
                            Literal::AssocList(vec![
                                (
                                    Literal::Path("/obj/item/device/geiger"),
                                    Literal::Number(4.)
                                ),
                                (
                                    Literal::Path("/obj/item/clothing/glasses/meson"),
                                    Literal::Number(2.)
                                )
                            ])
                        ),
                        ("req_access", Literal::List(vec![Literal::Number(301.)])),
                        ("req_log_access", Literal::Number(301.))
                    ]),
                ),
                ("/turf/simulated/floor/tiled/techfloor/grid", None),
                ("/area/talon_v2/engineering/star_store", None),
            ]
        )
    }

    #[test]
    fn test_empty_list() {
        let mut list = Located::new(
            r#""bd" = (
/obj/structure/closet/secure_closet/guncabinet/sidearm{
	anchored = 1;
	starts_with = list()
	})"#,
        );
        let (key, prefabs) = parse_prefab_line.parse_next(&mut list).unwrap();

        assert_eq!(key, "bd");
        assert_eq!(
            prefabs,
            vec![(
                "/obj/structure/closet/secure_closet/guncabinet/sidearm",
                Some(vec![
                    ("anchored", Literal::Number(1.)),
                    ("starts_with", Literal::List(vec![]))
                ])
            )]
        )
    }
}
