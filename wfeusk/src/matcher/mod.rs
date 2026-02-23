//! Word matching algorithm for finding all valid placements on a board.
//!
//! This module implements the core algorithm that finds all valid words that can be
//! played on a Scrabble-like board given a rack of letters. It scans each row and column,
//! using the DAWG to efficiently find valid word placements while respecting cross-word
//! constraints.

mod state;

use crate::board::{self, Pos};
use crate::letters::{Letters, Rack};
use crate::tile::Tile;
use crate::wordlist::Wordlist;

/// Finds all valid word placements on the board that can be made with the given letters.
///
/// This function scans all rows and columns on the board, finding every position where
/// a valid word can be placed using the available letters. It validates both the main
/// word and any crossing words formed perpendicular to it.
///
/// # Arguments
///
/// * `board` - The game board with existing tiles
/// * `wordlist` - Dictionary for validating words
/// * `letters` - Available letters as a string (e.g., "ABCDEFG")
///
/// # Returns
///
/// A vector of tuples containing:
/// - The word that can be played
/// - The starting position on the board
/// - The direction (HORIZONTAL or VERTICAL)
///
/// # Examples
///
/// ```no_run
/// use wfeusk::board::{Board, Pos, VERTICAL};
/// use wfeusk::matcher;
/// # use libdawg::dawg::builder::build_dawg;
/// # use wfeusk::wordlist::Wordlist;
/// # use typed_arena::Arena;
///
/// # let arena = Arena::new();
/// # let wordlist = Wordlist::new(build_dawg(&arena, ["HELLO", "WORLD"]).unwrap());
/// let mut board = Board::default();
/// board.play_word(&Pos::new(7, 7), VERTICAL, "HELLO");
///
/// let matches = matcher::find_all_words(&board, &wordlist, "WORLD");
/// for (word, pos, dir) in matches {
///     println!("Can play '{}' at {:?} {:?}", word, pos, dir);
/// }
/// ```
pub fn find_all_words(
    board: &board::Board,
    wordlist: &Wordlist<'_>,
    letters: &str,
) -> Vec<(String, Pos, board::Direction)> {
    let letters: Rack = letters.chars().collect();
    (0..15)
        .flat_map(|i| {
            find_row_words(board, wordlist, &letters, Pos::new(0, i), board::HORIZONTAL)
                .into_iter()
                .chain(find_row_words(
                    board,
                    wordlist,
                    &letters,
                    Pos::new(i, 0),
                    board::VERTICAL,
                ))
        })
        .collect()
}

fn find_row_words(
    board: &board::Board,
    wordlist: &Wordlist<'_>,
    letters: &Rack,
    start_pos: Pos,
    dir: board::Direction,
) -> Vec<(String, Pos, board::Direction)> {
    let mut rowdata: Vec<PosData> = (0..15)
        .map(|i| {
            let pos = start_pos + dir * i;
            PosData {
                letter: board.get(&pos).tile,
                valid_chars: get_valid_chars(board, wordlist, &pos, dir.flip()),
                connected: board.is_connected(&pos),
            }
        })
        .collect();
    match_words(wordlist, &mut rowdata, letters.clone())
        .map(|(word, offset)| {
            let start_pos = start_pos + dir * offset;
            (word, start_pos, dir)
        })
        .collect()
}

/// Finds all words that can be formed in a single row/column given position data and a rack.
///
/// This is a lower-level function used internally by [`find_all_words`]. It matches words
/// against pre-computed position data that includes valid characters at each position based
/// on crossing words.
///
/// # Arguments
///
/// * `wordlist` - Dictionary for word validation
/// * `rowdata` - Mutable slice of position data for the row/column
/// * `rack` - Available tiles to play
///
/// # Returns
///
/// Iterator yielding tuples of (word, starting_offset) for each valid placement.
pub fn match_words<'w>(
    wordlist: &Wordlist<'w>,
    rowdata: &'w mut [PosData],
    rack: Rack,
) -> impl Iterator<Item = (String, usize)> + 'w {
    state::RowMatcher::new(rowdata, rack, wordlist.get_root())
}

/// Determines which characters are valid to place at a given position.
///
/// Computes the set of letters that can be legally placed at a position by considering:
/// - If the position is occupied, only that letter is valid
/// - If empty, checks perpendicular crossing words to determine valid letters
/// - Uses the wordlist to ensure crossing words would be valid
///
/// # Arguments
///
/// * `board` - The game board
/// * `wordlist` - Dictionary for validating crossing words
/// * `pos` - Position to check
/// * `dir` - Direction of the word being played (crossing words are perpendicular)
///
/// # Returns
///
/// A `Letters` set containing all valid characters for this position.
pub fn get_valid_chars(
    board: &board::Board,
    wordlist: &Wordlist<'_>,
    pos: &Pos,
    dir: board::Direction,
) -> Letters {
    if let Some(tile) = board.get(pos).tile {
        match tile {
            Tile::Letter(ch) => Letters::one(ch),
            Tile::Wildcard(None) => Letters::any(),
            Tile::Wildcard(Some(ch)) => Letters::one(ch),
        }
    } else {
        match board.get_surrounding_letters(pos, dir) {
            Some(bi) => wordlist.valid_letters(bi.map(|bi| bi.tile.map(|tile| tile.into()))),
            None => Letters::any(),
        }
    }
}

/// Information about a single position on the board for word matching.
///
/// Contains pre-computed data about what letters are valid at this position based on
/// crossing words, whether the position is occupied, and whether it's connected to
/// existing tiles.
#[derive(Clone, Debug)]
pub struct PosData {
    /// Letters that can be legally placed here (considering crossing words)
    pub valid_chars: Letters,
    /// The tile currently at this position, if any
    pub letter: Option<Tile>,
    /// Whether this position is adjacent to an existing tile
    pub connected: bool,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::board::{HORIZONTAL, VERTICAL};
    use crate::wordlist::Wordlist;
    use libdawg::dawg::builder::build_dawg;
    use typed_arena::Arena;

    const WORDS: [&str; 34] = [
        "AL",
        "ALE",
        "ALES",
        "BAS",
        "BE",
        "BES",
        "BLOMKÅLS",
        "EK",
        "EL",
        "ESS",
        "KÅLS",
        "LE",
        "LES",
        "LESS",
        "OS",
        "OSS",
        "PASS",
        "PASSA",
        "PASSAR",
        "PASSARE",
        "PÅ",
        "SAL",
        "SM",
        "SO",
        "SOS",
        "SPA",
        "SPASM",
        "SPEL",
        "SPÅ",
        "SPÅS",
        "SÅ",
        "SÅS",
        "ÅLS",
        "ÅS",
    ];

    #[test]
    fn board_matches() {
        let letters = "PASSARE";
        let mut board = board::Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        let arena = Arena::new();
        let wordlist = Wordlist::new(build_dawg(&arena, WORDS).unwrap());
        let matches = super::find_all_words(&board, &wordlist, letters);
        assert!(matches.len() >= WORDS.len());
    }

    #[test]
    fn letters_are_not_modified() {
        let letters: Rack = "PASSARE".chars().collect();
        let original = letters.clone();
        let mut board = board::Board::default();
        board.play_word(&Pos::new(7, 7), board::VERTICAL, "BLOMKÅL");
        let arena = Arena::new();
        let wordlist = Wordlist::new(build_dawg(&arena, WORDS).unwrap());
        for i in 0..15 {
            super::find_row_words(&board, &wordlist, &letters, Pos::new(0, i), HORIZONTAL);
            super::find_row_words(&board, &wordlist, &letters, Pos::new(i, 0), VERTICAL);
            assert_eq!(letters, original);
        }
    }
}
