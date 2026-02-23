use itertools::Itertools;
use smallvec::{smallvec, SmallVec};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tile {
    /// A tile with a given letter.
    Letter(char),
    /// A wildcard tile, if it has been put down it has taken on the contained letter, otherwise
    /// it contains None.
    Wildcard(Option<char>),
}

type MatchVecType = SmallVec<[Tile; 4]>;

impl Tile {
    /// Returns true if this tile is a wildcard.
    pub(crate) fn is_wildcard(&self) -> bool {
        matches!(self, Tile::Wildcard(_))
    }

    /// Expand the tile according to which chars are valid to put in a specific square.
    /// - If tile is a letter that is not among valid_letters, it expands to nothing.
    /// - If tile is a letter that is among valid_letters, it expands to itself.
    /// - If tile is a wildcard (unassigned), it expands to a wildcard for each valid letter.
    /// - If tile is a wildcard that's already assigned, it expands to nothing (this shouldn't
    ///   happen in normal usage since only unassigned wildcards should be expanded).
    pub fn expand<I: Iterator<Item = char>>(
        &self,
        mut valid_chars: I,
    ) -> impl Iterator<Item = Tile> {
        let matching_tiles: MatchVecType = match self {
            Tile::Letter(ch) => {
                if valid_chars.contains(ch) {
                    smallvec![*self]
                } else {
                    smallvec![]
                }
            }
            Tile::Wildcard(None) => valid_chars.map(|ch| Tile::Wildcard(Some(ch))).collect(),
            Tile::Wildcard(Some(ch)) => {
                panic!("Attempted to expand already-assigned wildcard '{ch}'")
            }
        };
        matching_tiles.into_iter()
    }
}

impl From<char> for Tile {
    fn from(ch: char) -> Self {
        match ch {
            '*' => Tile::Wildcard(None),
            ch => Tile::Letter(ch),
        }
    }
}

impl From<Tile> for char {
    fn from(value: Tile) -> Self {
        match value {
            Tile::Letter(ch) => ch,
            Tile::Wildcard(None) => '*',
            Tile::Wildcard(Some(ch)) => ch,
        }
    }
}

#[cfg(test)]
mod test {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn is_wildcard() {
        let tile: Tile = '*'.into();
        assert!(tile.is_wildcard())
    }

    #[test]
    fn is_not_wildcard() {
        for ch in "ABCDEFGHILJKLMNOPQRSTUVWXYZÅÄÖ".chars() {
            let tile: Tile = ch.into();
            assert!(!tile.is_wildcard())
        }
    }

    #[test]
    fn letter_into_char() {
        let expected = [
            ('A', Tile::Letter('A')),
            ('Ö', Tile::Letter('Ö')),
            ('鱼', Tile::Letter('鱼')),
        ];
        for (ch, tile) in expected {
            let tile_ch: char = tile.into();
            assert_eq!(tile_ch, ch)
        }
    }

    #[test]
    fn wildcard_into_char() {
        let ch: char = Tile::Wildcard(None).into();
        assert_eq!(ch, '*')
    }

    #[test]
    fn letter_from_char() {
        let expected = [
            ('A', Tile::Letter('A')),
            ('Ö', Tile::Letter('Ö')),
            ('鱼', Tile::Letter('鱼')),
        ];
        for (ch, tile) in expected {
            let ch_tile: Tile = ch.into();
            assert_eq!(ch_tile, tile)
        }
    }

    #[test]
    fn wildcard_from_char() {
        let ch_tile: Tile = '*'.into();
        assert_eq!(ch_tile, Tile::Wildcard(None))
    }

    #[test]
    fn expand_wildcard_tile() {
        let expanded = Tile::Wildcard(None).expand("ABCD".chars()).collect_vec();
        assert_eq!(
            expanded,
            vec![
                Tile::Wildcard(Some('A')),
                Tile::Wildcard(Some('B')),
                Tile::Wildcard(Some('C')),
                Tile::Wildcard(Some('D')),
            ]
        )
    }

    #[test]
    fn expand_letter_tile() {
        let expanded = Tile::Letter('B').expand("ABCD".chars()).collect_vec();
        assert_eq!(expanded, vec![Tile::Letter('B'),])
    }

    #[test]
    fn expand_letter_tile_not_matching() {
        let expanded = Tile::Letter('B').expand("ACDEFG".chars()).collect_vec();
        assert_eq!(expanded, vec![])
    }

    #[test]
    #[should_panic(expected = "already-assigned wildcard")]
    fn expand_wildcard_letter_panics() {
        let _ = Tile::Wildcard(Some('A'))
            .expand("ABCD".chars())
            .collect_vec();
    }

    #[test]
    fn size_of_match_vec_type() {
        assert!(std::mem::size_of::<MatchVecType>() <= 40);
    }
}
