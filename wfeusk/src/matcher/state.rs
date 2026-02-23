use super::PosData;
use crate::letters::Rack;
use crate::tile::Tile;
use libdawg::dawg::DawgNode;

pub(super) struct RowMatcher<'a, 'd: 'a> {
    start_pos: usize,
    rowdata: &'a mut [PosData],
    rack: Rack,
    letters_placed: Vec<PlacedTile<'d>>,
    root: &'d DawgNode<'d, char>,
}

#[derive(Clone, Debug)]
struct PlacedTile<'d> {
    board_pos: usize,
    rack_pos: usize,
    tile: Tile,
    node: &'d DawgNode<'d, char>,
}

impl<'a, 'd> RowMatcher<'a, 'd> {
    pub fn new(rowdata: &'a mut [PosData], rack: Rack, root: &'d DawgNode<'d, char>) -> Self {
        let num_tiles = rack.max_tiles();
        Self {
            start_pos: 0,
            rowdata,
            rack,
            letters_placed: Vec::with_capacity(num_tiles),
            root,
        }
    }

    /// Place a tile in the first available space, if that makes a word, then return in, otherwise
    /// keep placing tiles until a word is made, or it is not possible to place more tiles, either
    /// because we're not in the dictionary any longer or we have run out of tiles.
    ///
    /// immediate is set to false on the first call to make sure that no match is returned until we
    /// have modified the board since the last match (or initial state).
    fn extend(&mut self, immediate: bool) -> Option<String> {
        // Find the next space.
        let path = self.rowdata[self.start_pos..]
            .iter()
            .take_while(|pd| pd.letter.is_some());

        // Walk the dictionary.
        if let Some(node) = path.clone().try_fold(self.root, |no, pos_data| {
            // We know that there is a letter here.
            debug_assert!(pos_data.letter.is_some());
            pos_data.letter.and_then(|tile| no.get(tile.into()))
        }) {
            let connected = path.clone().any(|rd| rd.connected);
            let pos = path.count() + self.start_pos;

            // Test if this is a word that can be played legally.
            if immediate && node.is_word() && connected {
                debug_assert!(!self.letters_placed.is_empty());
                return self.rowdata[self.start_pos..]
                    .iter()
                    .map(|pd| match pd.letter {
                        Some(Tile::Letter(ch)) => Some(ch),
                        Some(Tile::Wildcard(opt_ch)) => opt_ch,
                        None => None,
                    })
                    .take_while(|ch| ch.is_some())
                    .collect();
            }
            // We're still in the dictionary, place a tile if there is room left.
            if pos < self.rowdata.len() {
                let next_tile_opt = { self.expanded_tiles(node, pos).next() };
                if let Some(next_tile) = next_tile_opt {
                    self.place_tile(pos, next_tile, node);
                    return self.extend(true);
                }
            }
        }
        self.swap_last_tile()
    }

    /// Replace the last placed tile with the next possible tile, or remove it completely if no
    /// other tile is possible. The replacement tile will either be the next tile from the rack
    /// alphabetically, or if it's a wildcard, the same wildcard, but taking on the next possible
    /// letter alphabetically, according to what letters are possible in this square.
    fn swap_last_tile(&mut self) -> Option<String> {
        self.remove_last_tile().and_then(|pt| {
            let next_tile = self
                .expanded_tiles(pt.node, pt.board_pos)
                .find(|ch| *ch > pt.tile);
            match next_tile {
                Some(next_tile) => {
                    // Update the last char with the next in order.
                    self.place_tile(pt.board_pos, next_tile, pt.node);
                    self.extend(true)
                }
                None => self.swap_last_tile(),
            }
        })
    }

    /// Calculate the letters that can legally go into this position, i.e. letters that match both
    /// the start of the word with possible continuations in the dictionary, as well as any crossing
    /// word.
    fn expanded_tiles<'s>(
        &'s self,
        node: &'s DawgNode<'s, char>,
        pos: usize,
    ) -> impl Iterator<Item = Tile> + 's {
        let valid_chars = node
            .children()
            .map(|(ch, _node)| ch)
            .filter(move |ch| self.rowdata[pos].valid_chars.contains(*ch));
        self.rack.expand_wildcards(valid_chars)
    }

    /// Place a tile in the given, empty position and update letters_placed and rowdata.
    fn place_tile(&mut self, pos: usize, tile: Tile, node: &'d DawgNode<'d, char>) {
        debug_assert_ne!(tile, Tile::Wildcard(None));
        debug_assert_eq!(self.rowdata[pos].letter, None);
        debug_assert!(self
            .letters_placed
            .last()
            .map(|pt| pt.board_pos < pos)
            .unwrap_or(true));
        let rack_pos = self.rack.remove(tile);
        self.letters_placed.push(PlacedTile {
            board_pos: pos,
            tile,
            rack_pos,
            node,
        });
        self.rowdata[pos].letter = Some(tile);
    }

    /// Remove the last placed tile, add it to the rack. Returns the last letter, or None if no tile
    /// was placed.
    fn remove_last_tile(&mut self) -> Option<PlacedTile<'d>> {
        let last = self.letters_placed.pop();
        if let Some(pt) = &last {
            // Put the letter back in the rack.
            self.rack.set(pt.tile, pt.rack_pos);
            self.rowdata[pt.board_pos].letter = None;
        }
        last
    }
}

impl<'a, 'd> Iterator for RowMatcher<'a, 'd> {
    type Item = (String, usize);
    /// Find the next match.
    fn next(&mut self) -> Option<(String, usize)> {
        while self.start_pos < self.rowdata.len() {
            if let Some(next) = self.extend(false) {
                return Some((next, self.start_pos));
            }
            // We can only start a new word at this position if the tile before is empty, so advance
            // start_pos by the number of non-empty squares.
            let non_empty = self.rowdata[self.start_pos..]
                .iter()
                .take_while(|rd| rd.letter.is_some())
                .count();
            self.start_pos += non_empty + 1;
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::letters;
    use crate::letters::Letters;
    use crate::wordlist_test::WORDLIST;
    use libdawg::dawg::builder::build_dawg;
    use std::collections::BTreeSet;

    fn make_row(s: &str) -> Vec<(char, &str)> {
        s.chars()
            .map(|ch| {
                let valid = if ch == ' ' { "*" } else { "" };
                (ch, valid)
            })
            .collect()
    }

    #[test]
    fn matches() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row("K ASAR   ");
        test_row(
            &row,
            "ALNS",
            &wordlist,
            &["KLASAR", "KLASARNA", "KLASARNAS", "KLASARS", "KNASAR"],
        );
    }

    #[test]
    fn matches_with_start_pos() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row(" K ASAR   ");
        test_row(
            &row,
            "ALNS",
            &wordlist,
            &["KLASAR", "KLASARNA", "KLASARNAS", "KLASARS", "KNASAR"],
        );
    }

    #[test]
    fn matches_after_blocker_and_one_space() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row("X K ASAR   ");
        test_row(
            &row,
            "ALNS",
            &wordlist,
            &["KLASAR", "KLASARNA", "KLASARNAS", "KLASARS", "KNASAR"],
        );
    }

    #[test]
    fn matches_between_blockers() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row("X K ASAR X ");
        test_row(&row, "ALNS", &wordlist, &["KLASAR", "KNASAR"]);
    }

    #[test]
    fn matches_with_high_start_pos() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row("      K ASAR   ");
        test_row(
            &row,
            "ALNS",
            &wordlist,
            &["KLASAR", "KLASARNA", "KLASARNAS", "KLASARS", "KNASAR"],
        );
    }

    #[test]
    fn does_not_match_multiple_as_when_only_one_a_in_rack() {
        let wordlist = [
            "ASAR",
            "KLAS",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row("      KL S R   ");
        test_row(&row, "ALNS", &wordlist, &["KLAS"]);
    }

    #[test]
    fn matches_multiple_as_when_multiple_as_in_rack() {
        let wordlist = [
            "ASAR",
            "KLAS",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row("      KL S R   ");
        test_row(
            &row,
            "AAALNS",
            &wordlist,
            &["KLAS", "KLASAR", "KLASARNA", "KLASARNAS", "KLASARS"],
        );
    }

    #[test]
    fn matches_with_valid_chars() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = [
            (' ', "*"),
            ('K', ""),
            (' ', "L"),
            ('A', ""),
            ('S', ""),
            ('A', ""),
            ('R', ""),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
        ];
        test_row(
            &row,
            "ALNS",
            &wordlist,
            &["KLASAR", "KLASARNA", "KLASARNAS", "KLASARS"],
        );
    }

    #[test]
    fn matches_short() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row("K ASAR  ");
        test_row(
            &row,
            "ALNS",
            &wordlist,
            &["KLASAR", "KLASARNA", "KLASARS", "KNASAR"],
        );
    }

    #[test]
    fn word_end() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = make_row("K ASAR A ");
        test_row(
            &row,
            "ALNS",
            &wordlist,
            &["KLASAR", "KLASARNA", "KLASARNAS", "KNASAR"],
        );
    }

    #[test]
    fn empty_row_adjacent_to_word() {
        let row = [
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "EHIOUYÄ"),
            (' ', "EOYÄ"),
            (' ', "JKMRS"),
            (' ', "OUÅÖ"),
            (' ', "OÖ"),
            (' ', "HKLNRST"),
            (' ', "EOYÄ"),
            (' ', "*"),
        ];
        test_row(
            &row,
            "ABEERST",
            &WORDLIST,
            &["ARBETE", "BE", "ER", "REA", "RES", "SE", "SER"],
        );
    }

    #[test]
    fn matches_with_valid_chars_blocked_at_end() {
        let wordlist = [
            "ASAR",
            "KLASAR",
            "KLASARNA",
            "KLASARNAS",
            "KLASARS",
            "KNASAR",
        ];
        let row = [
            (' ', "*"),
            ('K', ""),
            (' ', "L"),
            ('A', ""),
            ('S', ""),
            ('A', ""),
            ('R', ""),
            (' ', "*"),
            (' ', "*"),
            ('K', ""),
        ];
        test_row(&row, "ALNS", &wordlist, &["KLASAR", "KLASARS"]);
    }

    #[test]
    fn empty_row_adjacent_to_word_wildcard() {
        let row = [
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "*"),
            (' ', "EHIOUYÄ"),
            (' ', "EOYÄ"),
            (' ', "JKMRS"),
            (' ', "OUÅÖ"),
            (' ', "OÖ"),
            (' ', "HKLNRST"),
            (' ', "EOYÄ"),
            (' ', "*"),
        ];
        let result = test_row_matches(&row, "ABERST", &WORDLIST);
        let expected = [
            ("BE", 6),
            ("ER", 8),
            ("ER", 13),
            ("REA", 12),
            ("RES", 12),
            ("SE", 6),
            ("SE", 12),
            ("SER", 12),
        ];

        assert_eq!(
            result,
            expected
                .into_iter()
                .map(|(s, p)| (s.to_owned(), p))
                .collect()
        );

        let result = test_row_matches(&row, "*ABERST", &WORDLIST);
        let expected_wildcard = [
            ("BE", 6),
            ("ER", 8),
            ("ER", 13),
            ("LE", 6),
            ("LE", 12),
            ("LER", 12),
            ("LES", 12),
            ("OR", 8),
            ("OR", 11),
            ("OR", 13),
            ("OS", 8),
            ("OS", 11),
            ("OS", 13),
            ("REA", 12),
            ("RES", 12),
            ("SE", 6),
            ("SE", 12),
            ("SER", 12),
            ("SES", 12),
            ("ÅSE", 5),
        ];

        assert_eq!(
            result,
            expected_wildcard
                .into_iter()
                .map(|(s, p)| (s.to_owned(), p))
                .collect()
        );
    }

    fn test_row(row: &[(char, &str)], rack: &str, wordlist: &[&str], expected: &[&str]) {
        let results: BTreeSet<_> = test_row_matches(row, rack, wordlist)
            .into_iter()
            .map(|(s, _)| s)
            .collect();
        assert_eq!(
            results,
            expected
                .iter()
                .map(|s| s.to_string())
                .collect::<BTreeSet<String>>()
        );
    }

    fn test_row_matches(
        row: &[(char, &str)],
        rack: &str,
        wordlist: &[&str],
    ) -> BTreeSet<(String, usize)> {
        let rack: letters::Rack = rack.into();
        let arena = typed_arena::Arena::new();
        let root = build_dawg(&arena, wordlist).unwrap();
        let mut rowdata: Vec<_> = row
            .iter()
            .map(|(ch, allowed)| (*ch, allowed.chars().collect::<letters::Letters>()))
            .map(|(ch, valid)| PosData {
                letter: (ch != ' ').then_some(ch.into()),
                connected: valid != Letters::any(),
                valid_chars: valid.clone(),
            })
            .collect();
        RowMatcher::new(&mut rowdata, rack, root).collect()
    }
}
