pub mod points;

use itertools::Itertools;
use std::collections::BTreeSet;

use crate::tile::Tile;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Letters {
    container: LettersContainer,
}

impl Letters {
    /// Creates an empty Letters.
    pub const fn empty() -> Self {
        Letters {
            container: LettersContainer::Empty,
        }
    }

    /// Creates a new Letters containing the characters in the given Iterator.
    pub fn new<I: Iterator<Item = char>>(letter_iter: &mut I) -> Self {
        letter_iter.collect()
    }

    /// Creates a Letters containing only the given char.
    pub const fn one(ch: char) -> Self {
        Letters {
            container: LettersContainer::One(ch),
        }
    }

    /// Creates a Letters containing every possible character.
    pub const fn any() -> Self {
        Letters {
            container: LettersContainer::Any,
        }
    }

    /// Returns true if this Letters is empty.
    pub fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

    /// Returns true if this Letters is not empty.
    pub fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }

    /// Returns true if this Letters contains the given character.
    pub fn contains(&self, ch: char) -> bool {
        self.container.contains(ch)
    }

    /// Returns an iterator that yields all letters from the input iterator that are in this
    /// Letters.
    pub fn intersects<'a, I: Iterator<Item = char> + 'a>(
        &'a self,
        char_iter: I,
    ) -> impl Iterator<Item = char> + 'a {
        self.container.intersects(char_iter)
    }

    /// Returns true if the given Tile matches this Letters.
    pub fn matches_tile(&self, tile: Tile) -> bool {
        match tile {
            Tile::Letter(ch) => self.contains(ch),
            Tile::Wildcard(Some(ch)) => self.contains(ch),
            Tile::Wildcard(None) => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LettersContainer {
    Empty,
    One(char),
    Some(BTreeSet<char>),
    Any,
}

impl LettersContainer {
    /// Return `true` if there are no letters in the container.
    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Return `true` if `ch` is in the container.
    fn contains(&self, ch: char) -> bool {
        assert!(
            ch != '*',
            "Wildcard is not a valid character to check for containment"
        );
        match self {
            Self::Empty => false,
            Self::One(letter) => *letter == ch,
            Self::Some(letters) => letters.contains(&ch),
            Self::Any => true,
        }
    }

    /// Returns an iterator over the characters that are both in this and in char_iter.
    fn intersects<'a, I: Iterator<Item = char> + 'a>(
        &'a self,
        char_iter: I,
    ) -> impl Iterator<Item = char> + 'a {
        char_iter.filter(move |ch| self.contains(*ch))
    }
}

impl FromIterator<char> for Letters {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        let container = {
            // Avoid creating a BTreeSet unless we have at least two items in iter.
            if let Some(ch1) = iter.next() {
                if let Some(ch2) = iter.next() {
                    let letters = IntoIterator::into_iter([ch1, ch2]).chain(iter).collect();
                    LettersContainer::Some(letters)
                } else if ch1 == '*' {
                    LettersContainer::Any
                } else {
                    LettersContainer::One(ch1)
                }
            } else {
                LettersContainer::Empty
            }
        };
        Letters { container }
    }
}

impl From<&str> for Letters {
    fn from(s: &str) -> Self {
        s.chars().collect()
    }
}

impl std::fmt::Display for Letters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.container {
            LettersContainer::Empty => Ok(()),
            LettersContainer::One(letter) => write!(f, "{letter}"),
            LettersContainer::Some(letters) => {
                for ch in letters {
                    write!(f, "{ch}")?;
                }
                Ok(())
            }
            LettersContainer::Any => write!(f, "*"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rack {
    letters: Vec<Option<Tile>>,
}

impl Rack {
    pub fn new(letters: Vec<Tile>) -> Self {
        let tiles = letters.into_iter().map(Some).collect_vec();
        debug_assert!(Rack {
            letters: tiles.clone()
        }
        .is_sorted());
        Rack { letters: tiles }
    }

    /// Return an iterator over the letters that occur both in this Rack and in the given iterator.
    pub fn intersect<'a>(&'a self, valid_letters: &'a Letters) -> impl Iterator<Item = Tile> + 'a {
        self.iter().dedup().filter(|tile| match tile {
            Tile::Letter(ch) => valid_letters.contains(*ch),
            Tile::Wildcard(_) => true,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = Tile> + '_ {
        self.letters.iter().filter_map(|tile| *tile)
    }

    pub fn remove(&mut self, tile: Tile) -> usize {
        // If tile is a Wildcard(ch), we need to look for a Wildcard(None) instead, since those are
        // the only ones that can be in the rack.
        let matching_tile = if tile.is_wildcard() {
            Some(Tile::Wildcard(None))
        } else {
            Some(tile)
        };
        let pos = self
            .letters
            .iter()
            .position(|t| *t == matching_tile)
            .expect("Attempted to remove a tile that wasn't in the rack");
        self.letters[pos] = None;
        pos
    }

    pub fn set(&mut self, tile: Tile, pos: usize) {
        debug_assert!(self.letters[pos].is_none());
        // If this is a wildcard, once it's back in the rack it's a blank wildcard.
        let tile = if tile.is_wildcard() {
            Tile::Wildcard(None)
        } else {
            tile
        };
        self.letters[pos] = Some(tile);
    }

    /// Returns true if the Rack is sorted. This is only used for testing since racks always have to
    /// be sorted.
    pub(crate) fn is_sorted(&self) -> bool {
        let i1 = self.letters.iter().filter_map(|tile| *tile);
        let i2 = i1.clone().skip(1);
        i1.zip(i2).all(|(ch1, ch2)| ch1 <= ch2)
    }

    /// Returns an iterator over the unique letters in this Rack, as well as the remaining Rack
    /// after removing that letter.
    pub fn unique_letters(&self) -> impl Iterator<Item = (Tile, Rack)> + '_ {
        self.letters
            .iter()
            .enumerate()
            .filter_map(|(i, tile)| tile.map(|t| (i, t)))
            .dedup_by(|(_, ch1), (_, ch2)| ch1 == ch2)
            .map(|(pos, tile)| {
                let mut next_letters = self.letters.clone();
                next_letters.remove(pos);
                (
                    tile,
                    Rack {
                        letters: next_letters,
                    },
                )
            })
    }

    /// Returns the maximum number of tiles this rack can hold (including removed slots).
    pub(crate) fn max_tiles(&self) -> usize {
        self.letters.len()
    }

    pub fn contains(&self, tile: Tile) -> bool {
        self.iter().any(|t| t == tile)
    }

    /// Expands wildcards in this rack from blank wildcard into wildcards taking on every different
    /// possible letter given by crossing words, and possible follow up letters in the dictionary.
    /// While crossing words can be wildcards (no crossing letter, so no restrictions), the
    /// dictionary always yields a finite number of actual letters, so valid_chars is just an
    /// iterator over char instead of Tile. Normal letter tiles are just expanded into themselves.
    pub fn expand_wildcards<'a>(
        &'a self,
        valid_chars: impl Iterator<Item = char> + Clone + 'a,
    ) -> impl Iterator<Item = Tile> + 'a {
        self.iter()
            .flat_map(move |tile| tile.expand(valid_chars.clone()))
    }
}

impl From<&str> for Rack {
    fn from(s: &str) -> Self {
        s.chars().collect()
    }
}

impl FromIterator<char> for Rack {
    fn from_iter<I: IntoIterator<Item = char>>(into_iter: I) -> Rack {
        Rack {
            letters: into_iter
                .into_iter()
                .sorted_unstable()
                .map(|ch| Some(ch.into()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn tile_vec(chars: &[char]) -> Vec<Option<Tile>> {
        chars.iter().map(|ch| Some(Tile::Letter(*ch))).collect()
    }

    #[test]
    fn rack_is_sorted() {
        let rack = Rack::from("PQABC");
        assert!(rack.is_sorted());
        assert_eq!(rack.letters, tile_vec(&['A', 'B', 'C', 'P', 'Q']));
    }

    #[test]
    fn rack_is_not_sorted() {
        let rack = Rack {
            letters: vec![
                Some(Tile::Letter('A')),
                Some(Tile::Letter('C')),
                Some(Tile::Letter('B')),
            ],
        };
        assert!(!rack.is_sorted());
    }

    #[test]
    fn rack_is_sorted_none_between() {
        let rack = Rack {
            letters: vec![
                Some(Tile::Letter('A')),
                Some(Tile::Letter('B')),
                None,
                Some(Tile::Letter('B')),
                None,
                None,
                Some(Tile::Letter('C')),
                Some(Tile::Letter('D')),
                Some(Tile::Letter('D')),
                None,
            ],
        };
        assert!(rack.is_sorted());
    }

    #[test]
    fn rack_is_not_sorted_none_between() {
        let rack = Rack {
            letters: vec![
                Some(Tile::Letter('A')),
                Some(Tile::Letter('B')),
                None,
                Some(Tile::Letter('A')),
                None,
                None,
                Some(Tile::Letter('C')),
                Some(Tile::Letter('D')),
                Some(Tile::Letter('D')),
                None,
            ],
        };
        assert!(!rack.is_sorted());
    }

    #[test]
    fn rack_is_sorted_double_char() {
        let rack = Rack::from("PQCABC");
        assert!(rack.is_sorted());
        assert_eq!(rack.letters, tile_vec(&['A', 'B', 'C', 'C', 'P', 'Q']));
    }

    #[test]
    fn rack_is_sorted_multiple_chars() {
        let rack = Rack::from("PQCAPPCCBC");
        assert!(rack.is_sorted());
        assert_eq!(
            rack.letters,
            tile_vec(&['A', 'B', 'C', 'C', 'C', 'C', 'P', 'P', 'P', 'Q'])
        );
    }

    #[test]
    fn letters_sorted() {
        let letters: Letters = "PQABC".chars().collect();
        assert_eq!(letters.to_string(), "ABCPQ");
    }

    #[test]
    fn intersect_one() {
        let valid: Letters = "L".into();
        let rack = Rack::from("ABCLMNO");
        let intersection: String = rack.intersect(&valid).map(Into::<char>::into).collect();
        assert_eq!(intersection, "L");
    }

    #[test]
    fn intersect_several() {
        let valid: Letters = "ALOPQRSTUVWXYZ".chars().collect();
        let rack = Rack::from("ABCLMNO");
        let intersection: String = rack.intersect(&valid).map(Into::<char>::into).collect();
        assert_eq!(intersection, "ALO");
    }

    #[test]
    fn intersect_several_unsorted_rack() {
        let valid: Letters = "ALOPQRSTUVWXYZ".chars().collect();
        let rack = Rack::from("COALMNB");
        let intersection: String = rack.intersect(&valid).map(Into::<char>::into).collect();
        assert_eq!(intersection, "ALO");
    }

    #[test]
    fn intersect_several_nonascii() {
        let valid: Letters = "ALOPQRSTUVWXYZÅÄÖ金".chars().collect();
        let rack = Rack::from("ABCLMNOÅÖ金");
        let intersection: String = rack.intersect(&valid).map(Into::<char>::into).collect();
        assert_eq!(intersection, "ALOÅÖ金");
    }

    #[test]
    fn intersect_wildcard() {
        let valid = Letters::any();
        let rack = Rack::from("ABCLMNO");
        let intersection: String = rack.intersect(&valid).map(Into::<char>::into).collect();
        assert_eq!(intersection, "ABCLMNO");
    }

    #[test]
    fn intersect_empty() {
        let valid = Letters::empty();
        let rack: Rack = "ABCLMNO".into();
        let intersection: String = rack.intersect(&valid).map(Into::<char>::into).collect();
        assert_eq!(intersection, "");
    }

    #[test]
    fn unique_letters() {
        let rack: Rack = "ABCLMNO".chars().collect();
        let unique: Vec<_> = rack.unique_letters().collect();
        assert_eq!(
            unique,
            vec![
                (Tile::Letter('A'), "BCLMNO".into()),
                (Tile::Letter('B'), "ACLMNO".into()),
                (Tile::Letter('C'), "ABLMNO".into()),
                (Tile::Letter('L'), "ABCMNO".into()),
                (Tile::Letter('M'), "ABCLNO".into()),
                (Tile::Letter('N'), "ABCLMO".into()),
                (Tile::Letter('O'), "ABCLMN".into()),
            ]
        )
    }

    #[test]
    fn unique_letters_double_should_remain() {
        let rack = Rack::from("ABCCMNO");
        let unique: Vec<_> = rack.unique_letters().collect();
        assert_eq!(
            unique,
            vec![
                (Tile::Letter('A'), "BCCMNO".into()),
                (Tile::Letter('B'), "ACCMNO".into()),
                (Tile::Letter('C'), "ABCMNO".into()),
                (Tile::Letter('M'), "ABCCNO".into()),
                (Tile::Letter('N'), "ABCCMO".into()),
                (Tile::Letter('O'), "ABCCMN".into()),
            ]
        )
    }

    #[test]
    fn unique_letters_several() {
        let rack = Rack::from("AAAAAAA");
        let unique: Vec<(Tile, Rack)> = rack.unique_letters().collect();
        assert_eq!(unique, vec![(Tile::Letter('A'), Rack::from("AAAAAA")),])
    }

    #[test]
    fn collect_rack_with_wildcard() {
        let rack = Rack::from("*");
        assert_eq!(rack.letters[0], Some(Tile::Wildcard(None)))
    }

    #[test]
    fn rack_contains() {
        let rack = Rack::from("ABC");
        assert!(rack.contains(Tile::Letter('A')));
        assert!(!rack.contains(Tile::Letter('D')));
    }

    #[test]
    fn expand_wildcards_letter() {
        let rack = Rack::from("CQ");
        let valid_chars = "CDQ".chars();
        let expanded: Vec<_> = rack.expand_wildcards(valid_chars).collect();
        assert_eq!(expanded, vec![Tile::Letter('C'), Tile::Letter('Q')]);
    }

    #[test]
    fn expand_wildcards_any_letters() {
        let rack = Rack::from("CDEQ");
        let valid_chars = "ACDP".chars();
        let expanded: Vec<_> = rack.expand_wildcards(valid_chars).collect();
        assert_eq!(expanded, vec![Tile::Letter('C'), Tile::Letter('D'),]);
    }

    #[test]
    fn expand_wildcards_wildcard() {
        let rack = Rack::from("AQ*");
        let valid_chars = "ACFH".chars();
        let expanded: Vec<_> = rack.expand_wildcards(valid_chars).collect();
        assert_eq!(
            expanded,
            vec![
                Tile::Wildcard(Some('A')),
                Tile::Wildcard(Some('C')),
                Tile::Wildcard(Some('F')),
                Tile::Wildcard(Some('H')),
                Tile::Letter('A'),
            ]
        );
    }
}
