//! This provides a grid struct which can be used to rotate a given tile grid before iterating over it in BYOND order
use array2d::Array2D;

/// Degrees to rotate the grid by.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Rotation {
    None,
    Ninety,
    OneEighty,
    TwoSeventy,
}

/// Helper struct for working with a 2d grid, primarily for rotating it by 90 degree increments.
pub struct Grid<'a> {
    bottom_left: (usize, usize, usize),
    array: Array2D<&'a str>,
}

impl<'a> Grid<'a> {
    /// Create a new grid at the given origin, from a list of lines in a dmm file.
    pub fn new(
        bottom_left: (usize, usize, usize),
        key_len: usize,
        block: &[&'a str],
    ) -> eyre::Result<Self> {
        let iterator = block.iter().flat_map(|line| separate_turfs(line, key_len));
        let num_rows = block.len();
        let num_columns = separate_turfs(block[0], key_len).count();

        let array = Array2D::from_iter_row_major(iterator, num_rows, num_columns)?;

        Ok(Self { bottom_left, array })
    }

    /// Rotates the grid and returns a Vec iterating over the coordinates in byond order.
    pub fn rotate(&self, rotation: Rotation) -> Vec<((usize, usize, usize), &str)> {
        let num_rows = self.array.num_rows();
        let num_columns = self.array.num_columns();

        match rotation {
            Rotation::None => self
                .array
                .enumerate_row_major()
                .map(|((y, x), s)| {
                    (
                        (
                            self.bottom_left.0 + x,
                            self.bottom_left.1 + (num_rows - y - 1),
                            self.bottom_left.2,
                        ),
                        *s,
                    )
                })
                .collect(),
            Rotation::Ninety => self
                .array
                .columns_iter()
                .rev()
                .enumerate()
                .flat_map(move |(y, s)| {
                    s.enumerate().map(move |(x, s)| {
                        (
                            (
                                self.bottom_left.0 + x,
                                self.bottom_left.1 + (num_columns - y - 1),
                                self.bottom_left.2,
                            ),
                            *s,
                        )
                    })
                })
                .collect(),
            Rotation::OneEighty => self
                .array
                .enumerate_row_major()
                .rev()
                .map(|((y, x), s)| {
                    (
                        (
                            self.bottom_left.0 + (num_columns - x - 1),
                            self.bottom_left.1 + y,
                            self.bottom_left.2,
                        ),
                        *s,
                    )
                })
                .collect(),
            Rotation::TwoSeventy => self
                .array
                .columns_iter()
                .enumerate()
                .flat_map(move |(y, s)| {
                    s.rev().enumerate().map(move |(x, s)| {
                        (
                            (
                                self.bottom_left.0 + x,
                                self.bottom_left.1 + (num_columns - y - 1),
                                self.bottom_left.2,
                            ),
                            *s,
                        )
                    })
                })
                .collect(),
        }
    }
}

fn separate_turfs(mut s: &str, n: usize) -> impl Iterator<Item = &'_ str> {
    assert_ne!(n, 0);
    std::iter::from_fn(move || {
        let index = s
            .char_indices()
            .nth(n)
            .map(|(index, _)| index)
            .unwrap_or(s.len());
        let (item, rest) = s.split_at(index);
        if item.is_empty() {
            None
        } else {
            s = rest;
            Some(item)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tgm() {
        /* a
         * b
         * c
         */
        let grid = Grid::new((1, 1, 1), 1, &["a", "b", "c"]).unwrap();

        /* a
         * b
         * c
         */
        assert_eq!(
            grid.rotate(Rotation::None),
            vec![((1, 3, 1), "a"), ((1, 2, 1), "b"), ((1, 1, 1), "c")]
        );

        /*
         * c
         * b
         * a
         */
        assert_eq!(
            grid.rotate(Rotation::OneEighty),
            vec![((1, 3, 1), "c"), ((1, 2, 1), "b"), ((1, 1, 1), "a")]
        );

        /*
         * a b c
         */
        assert_eq!(
            grid.rotate(Rotation::Ninety),
            vec![((1, 1, 1), "a"), ((2, 1, 1), "b"), ((3, 1, 1), "c")]
        );

        /*
         * c b a
         */
        assert_eq!(
            grid.rotate(Rotation::TwoSeventy),
            vec![((1, 1, 1), "c"), ((2, 1, 1), "b"), ((3, 1, 1), "a")]
        )
    }

    #[test]
    fn test_iteration_order() {
        let map = vec!["abc", "def", "ghi"];
        let grid = Grid::new((1, 1, 1), 1, &map).unwrap();

        // Easy ones first: 0deg
        assert_eq!(
            grid.rotate(Rotation::None),
            vec![
                ((1, 3, 1), "a"),
                ((2, 3, 1), "b"),
                ((3, 3, 1), "c"),
                ((1, 2, 1), "d"),
                ((2, 2, 1), "e"),
                ((3, 2, 1), "f"),
                ((1, 1, 1), "g"),
                ((2, 1, 1), "h"),
                ((3, 1, 1), "i"),
            ]
        );

        // 180deg
        assert_eq!(
            grid.rotate(Rotation::OneEighty),
            vec![
                ((1, 3, 1), "i"),
                ((2, 3, 1), "h"),
                ((3, 3, 1), "g"),
                ((1, 2, 1), "f"),
                ((2, 2, 1), "e"),
                ((3, 2, 1), "d"),
                ((1, 1, 1), "c"),
                ((2, 1, 1), "b"),
                ((3, 1, 1), "a"),
            ]
        );

        // Now the hard ones: 90deg
        assert_eq!(
            grid.rotate(Rotation::Ninety),
            vec![
                ((1, 3, 1), "c"),
                ((2, 3, 1), "f"),
                ((3, 3, 1), "i"),
                ((1, 2, 1), "b"),
                ((2, 2, 1), "e"),
                ((3, 2, 1), "h"),
                ((1, 1, 1), "a"),
                ((2, 1, 1), "d"),
                ((3, 1, 1), "g"),
            ]
        );

        // 270deg
        assert_eq!(
            grid.rotate(Rotation::TwoSeventy),
            vec![
                ((1, 3, 1), "g"),
                ((2, 3, 1), "d"),
                ((3, 3, 1), "a"),
                ((1, 2, 1), "h"),
                ((2, 2, 1), "e"),
                ((3, 2, 1), "b"),
                ((1, 1, 1), "i"),
                ((2, 1, 1), "f"),
                ((3, 1, 1), "c"),
            ]
        );
    }
}
