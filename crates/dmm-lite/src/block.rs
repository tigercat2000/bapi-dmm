use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use winnow::{
    ascii::{dec_uint, line_ending, space0},
    combinator::{delimited, opt, repeat, separated_pair, terminated},
    prelude::*,
    token::take_while,
};

pub fn parse_coords(i: &mut &str) -> PResult<(usize, usize, usize)> {
    delimited(
        '(',
        separated_pair(dec_uint, ',', separated_pair(dec_uint, ',', dec_uint)),
        ')',
    )
    .map(|(a, (b, c))| (a, b, c))
    .parse_next(i)
}

pub fn map_characters<'s>(i: &mut &'s str) -> PResult<&'s str> {
    take_while(1.., ('a'..='z', 'A'..='Z')).parse_next(i)
}

pub fn parse_map_lines<'s>(i: &mut &'s str) -> PResult<Vec<&'s str>> {
    delimited(
        ("{\"", opt(line_ending)),
        repeat(1.., terminated(map_characters, opt(line_ending))),
        (opt(line_ending), "\"}"),
    )
    .parse_next(i)
}

type Block<'s> = ((usize, usize, usize), Vec<&'s str>);
pub fn parse_block<'s>(i: &mut &'s str) -> PResult<Block<'s>> {
    separated_pair(
        parse_coords,
        delimited(space0, '=', space0),
        parse_map_lines,
    )
    .parse_next(i)
}

/// Used for multithreading: Uses a fast regex to get the starting location of every map block
pub fn get_block_locations(i: &str) -> Vec<usize> {
    let re = Regex::new(r#"\((\d+),(\d+),(\d+)\) = \{"#).unwrap();

    let mut results = vec![];
    for offset in re
        .captures_iter(i)
        .filter_map(|c| c.get(0).map(|f| f.start()))
    {
        results.push(offset);
    }

    results
}

pub fn multithreaded_parse_map_locations(i: &str) -> Vec<Block> {
    let locations = get_block_locations(i);

    locations
        .par_iter()
        .filter_map(|loc| {
            let mut substring = &i[*loc..];
            parse_block(&mut substring).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coords() {
        let mut coords = "(1,2,3)";
        let mut bigger_coords = "(100,241,2)";

        assert_eq!(parse_coords.parse_next(&mut coords), Ok((1, 2, 3)));
        assert_eq!(
            parse_coords.parse_next(&mut bigger_coords),
            Ok((100, 241, 2))
        );
    }

    #[test]
    fn test_parse_map_lines() {
        let mut map_lines = "{\"aaaaabaac\naabaacaaa\naacaabaaa\"}";

        assert_eq!(
            parse_map_lines.parse_next(&mut map_lines),
            Ok(vec!["aaaaabaac", "aabaacaaa", "aacaabaaa"])
        );
    }

    #[test]
    fn test_parse_block() {
        let mut block = "(1,1,1) = {\"aaaaabaac\naabaacaaa\"}";
        let mut tgm_block = "(1,1,1) = {\"\naaa\naab\naac\naab\naac\naaa\"}";

        assert_eq!(
            parse_block.parse_next(&mut block),
            Ok(((1, 1, 1), vec!["aaaaabaac", "aabaacaaa"]))
        );
        assert_eq!(
            parse_block.parse_next(&mut tgm_block),
            Ok(((1, 1, 1), vec!["aaa", "aab", "aac", "aab", "aac", "aaa"]))
        );
    }
}
