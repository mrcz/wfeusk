//! Board representation for Scrabble-like word games.
//!
//! Provides a 15×15 game board with bonus squares (double/triple letter and word scores),
//! tile placement, and scoring logic. The board supports querying positions, checking
//! connectivity, and calculating word scores.

use crate::letters::points::LETTER_POINTS;
use crate::tile::Tile;
use crate::util::mirror;
use fmt::Debug;
use itertools::Itertools;
use std::convert::TryInto;
use std::fmt::{Display, Write};
use std::{collections::BTreeMap, fmt};

/// The default board. Since it is symmetric, only one quadrant needs to be defined
const DEFAULT_QUARTER_BOARD: &[&str] = &[
    "3l -- -- -- 3w -- -- 2l",
    "-- 2l -- -- -- 3l -- --",
    "-- -- 2w -- -- -- 2l --",
    "-- -- -- 3l -- -- -- 2w",
    "3w -- -- -- 2w -- 2l --",
    "-- 3l -- -- -- 3l -- --",
    "-- -- 2l -- 2l -- -- --",
    "2l -- -- 2w -- -- -- --",
];

/// A position on the board represented by x (column) and y (row) coordinates.
///
/// Coordinates are 0-indexed, with (0,0) in the top-left corner.
/// Valid coordinates range from 0 to 14 (inclusive) for a standard 15×15 board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos {
    /// Column coordinate (0-14)
    pub x: i32,
    /// Row coordinate (0-14)
    pub y: i32,
}

impl Pos {
    /// Creates a new Pos, the given x and y values must be convertible to i32, otherwise this
    /// function panics.
    pub fn new<T: TryInto<i32>>(x: T, y: T) -> Self {
        Self {
            x: x.try_into().ok().expect("x must be convertible to i32"),
            y: y.try_into().ok().expect("y must be convertible to i32"),
        }
    }

    /// Returns this positions x- and y-coordinates as a tuple.
    pub fn xy(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

impl std::ops::Add<Direction> for Pos {
    type Output = Self;

    fn add(self, rhs: Direction) -> Self::Output {
        Pos::new(self.x + rhs.dx, self.y + rhs.dy)
    }
}

impl std::ops::AddAssign<Direction> for Pos {
    fn add_assign(&mut self, rhs: Direction) {
        self.x += rhs.dx;
        self.y += rhs.dy;
    }
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{},{}]", self.x, self.y)
    }
}

/// A direction for word placement on the board (horizontal or vertical).
///
/// Directions can be added to positions, negated, multiplied by scalars, and flipped
/// between horizontal and vertical orientations.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Direction {
    dx: i32,
    dy: i32,
}

impl Direction {
    /// Flips the direction from horizontal to vertical or vice versa.
    ///
    /// # Examples
    ///
    /// ```
    /// use wfeusk::board::{HORIZONTAL, VERTICAL};
    ///
    /// assert_eq!(HORIZONTAL.flip(), VERTICAL);
    /// assert_eq!(VERTICAL.flip(), HORIZONTAL);
    /// ```
    pub const fn flip(self) -> Direction {
        Direction {
            dx: self.dy,
            dy: self.dx,
        }
    }
}

impl std::ops::Neg for Direction {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            dx: -self.dx,
            dy: -self.dy,
        }
    }
}

impl std::ops::Mul<i32> for Direction {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self {
            dx: rhs * self.dx,
            dy: rhs * self.dy,
        }
    }
}

impl std::ops::Mul<usize> for Direction {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self::Output {
        let rhs: i32 = rhs.try_into().expect("rhs overflow");
        self * rhs
    }
}

impl Debug for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            VERTICAL => f.write_char('↧'),
            HORIZONTAL => f.write_char('↦'),
            Direction { dx, dy } => f.write_fmt(format_args!("Direction {{{dx}}},{{{dy}}}")),
        }
    }
}

/// Iterator over board squares in a specific direction.
///
/// Created by [`Board::get_board_iter`] to iterate over a sequence of squares
/// starting from a position and moving in a direction.
#[derive(Clone)]
pub struct BoardIter<'b> {
    board: &'b Board,
    pos: Pos,
    dir: Direction,
    length: i32,
}

impl Iterator for BoardIter<'_> {
    type Item = BoardSquare;
    fn next(&mut self) -> Option<BoardSquare> {
        if self.length == 0 {
            None
        } else {
            let ch = self.board.get(&self.pos);
            self.pos += self.dir;
            self.length -= 1;
            Some(ch)
        }
    }
}

/// Horizontal direction constant (left to right).
pub const HORIZONTAL: Direction = Direction { dx: 1, dy: 0 };

/// Vertical direction constant (top to bottom).
pub const VERTICAL: Direction = Direction { dx: 0, dy: 1 };

/// Represents a single square on the board.
///
/// Contains information about the tile (if any), bonus multipliers, and connectivity.
/// Letter bonuses multiply the points for a single letter, while word bonuses multiply
/// the entire word's score.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct BoardSquare {
    /// The tile placed on this square, if any
    pub tile: Option<Tile>,
    /// Multiplier for letter score (2 = double letter, 3 = triple letter, etc.)
    pub letter_bonus: i32,
    /// Multiplier for word score (2 = double word, 3 = triple word, etc.)
    pub word_bonus: i32,
    /// Whether this square is adjacent to an occupied square
    pub connected: bool,
}

impl BoardSquare {
    fn letter_bonus_char(&self) -> char {
        Self::i32_to_char(self.letter_bonus)
    }
    fn word_bonus_char(&self) -> char {
        Self::i32_to_char(self.word_bonus)
    }
    fn i32_to_char(val: i32) -> char {
        assert!((0..=9).contains(&val));
        let num: u32 = val.try_into().unwrap();
        char::from_digit(num, 10).unwrap()
    }
    /// Convert the BoardSquare to one unicode character, if there is a letter placed in the
    /// BoardSquare, that letter is returned, otherwise  if there is a bonus, it is returned -
    /// letter bonuses are written in subscript and word bonuses in superscript. If there is
    /// neither a letter nor bonus the empty square symbol is returned.
    const fn to_char(self) -> char {
        let lb = self.letter_bonus as usize;
        let wb = self.word_bonus as usize;
        if let Some(tile) = self.tile {
            match tile {
                Tile::Letter(ch) => ch,
                Tile::Wildcard(None) => '*',
                Tile::Wildcard(Some(ch)) => ch,
            }
        } else if lb != 1 {
            ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'][lb]
        } else if wb != 1 {
            ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'][wb]
        } else {
            '·'
        }
    }
}

impl fmt::Display for BoardSquare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (l, r) = if let Some(tile) = self.tile {
            (tile.into(), ' ')
        } else if self.letter_bonus != 1 {
            (self.letter_bonus_char(), 'l')
        } else if self.word_bonus != 1 {
            (self.word_bonus_char(), 'w')
        } else {
            ('-', '-')
        };
        write!(f, "{l}{r}")
    }
}

impl fmt::Debug for BoardSquare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// A 15×15 Scrabble-like game board with bonus squares and scoring.
///
/// The board stores tiles, tracks bonus multipliers, and provides methods for:
/// - Placing and querying tiles
/// - Checking connectivity and valid moves
/// - Calculating word scores
/// - Iterating over rows and columns
///
/// # Examples
///
/// ```
/// use wfeusk::board::{Board, Pos, VERTICAL};
///
/// let mut board = Board::default();
/// board.play_word(&Pos::new(7, 7), VERTICAL, "HELLO");
/// assert!(board.is_occupied(&Pos::new(7, 7)));
/// ```
pub struct Board {
    // Use 16 columns (power of 2) instead of 15 for faster array indexing.
    // Trades 180 bytes of memory for potential performance gain via bit-shift optimization.
    // Note: Only indices 0-14 are valid; column 15 is never accessed.
    squares: [[BoardSquare; 16]; 15],
    letter_points: BTreeMap<char, i32>,
}

impl Default for Board {
    /// Returns a default board with standard bonus squares
    fn default() -> Self {
        let mut squares: [[BoardSquare; 16]; 15] = [[BoardSquare {
            letter_bonus: 0,
            word_bonus: 0,
            tile: None,
            connected: false,
        }; 16]; 15];
        let lines = mirror(DEFAULT_QUARTER_BOARD.iter());
        for (y, line) in lines.enumerate() {
            let line_squares = parse_board_line(line).expect("Error parsing default board");
            for (x, square) in mirror(line_squares.into_iter()).enumerate() {
                squares[y][x] = square;
            }
        }
        squares[7][7].connected = true;
        let letter_points = LETTER_POINTS.iter().cloned().collect();
        Board {
            squares,
            letter_points,
        }
    }
}

impl Board {
    /// Gets the content of a board square
    pub const fn get(&self, pos: &Pos) -> BoardSquare {
        self.squares[pos.y as usize][pos.x as usize]
    }

    /// Returns true if the square is occupied
    pub fn is_occupied(&self, pos: &Pos) -> bool {
        Self::within_bounds(pos) && self.get(pos).tile.is_some()
    }

    /// Returns true if the square is free (within bounds and not occupied).
    pub fn is_free(&self, pos: &Pos) -> bool {
        Self::within_bounds(pos) && self.get(pos).tile.is_none()
    }

    /// Returns true if this square is connected to at least one occupied neighbor.
    ///
    /// A square is considered connected if any of its four orthogonal neighbors
    /// (up, down, left, right) contains a tile.
    pub fn is_connected(&self, pos: &Pos) -> bool {
        let (x, y) = pos.xy();
        [
            Pos::new(x - 1, y),
            Pos::new(x + 1, y),
            Pos::new(x, y - 1),
            Pos::new(x, y + 1),
        ]
        .iter()
        .any(|pos| self.is_occupied(pos))
    }

    /// Returns true if the position is within the 15x15 board.
    pub fn within_bounds(pos: &Pos) -> bool {
        (0..15).contains(&pos.x) && (0..15).contains(&pos.y)
    }

    /// Set the content of a square
    pub fn set(&mut self, pos: &Pos, tile: Tile) {
        debug_assert!(self.is_free(pos));
        self.squares[pos.y as usize][pos.x as usize].tile = Some(tile);
    }

    fn find_next_empty_index(&self, pos: &Pos, dir: Direction) -> i32 {
        let mut pos = *pos + dir;
        while self.is_occupied(&pos) {
            pos += dir;
        }
        if dir.dx != 0 {
            pos.x - dir.dx
        } else {
            pos.y - dir.dy
        }
    }

    fn start_end(&self, pos: &Pos, dir: Direction) -> (i32, i32) {
        let e = self.find_next_empty_index(pos, dir);
        let s = self.find_next_empty_index(pos, -dir);
        (s, e)
    }

    /// Gets the surrounding letters at a position in the given direction, including position.
    ///
    /// Returns the starting coordinates and an iterator over the contiguous word segment.
    /// Returns `None` if there are no surrounding letters (isolated position).
    pub fn get_surrounding_letters_pos(
        &'_ self,
        pos: &Pos,
        dir: Direction,
    ) -> Option<(i32, i32, BoardIter<'_>)> {
        let (s, e) = self.start_end(pos, dir);
        let length = e - s + 1;
        if length <= 1 {
            None
        } else {
            let (cx, cy) = if dir == HORIZONTAL {
                (s, pos.y)
            } else {
                (pos.x, s)
            };
            Some((cx, cy, self.get_board_iter(Pos::new(cx, cy), length, dir)))
        }
    }

    /// Gets an iterator over the surrounding letters at a position in the given direction.
    ///
    /// Returns `None` if there are no surrounding letters (isolated position).
    pub fn get_surrounding_letters(&'_ self, pos: &Pos, dir: Direction) -> Option<BoardIter<'_>> {
        self.get_surrounding_letters_pos(pos, dir)
            .map(|(_, _, word)| word)
    }

    /// Creates an iterator over board squares starting at a position.
    ///
    /// # Arguments
    ///
    /// * `pos` - Starting position
    /// * `length` - Number of squares to iterate over
    /// * `dir` - Direction to iterate (HORIZONTAL or VERTICAL)
    pub const fn get_board_iter(&'_ self, pos: Pos, length: i32, dir: Direction) -> BoardIter<'_> {
        BoardIter {
            board: self,
            pos,
            length,
            dir,
        }
    }

    /// Places a word on the board at the specified position and direction.
    ///
    /// # Arguments
    ///
    /// * `pos` - Starting position for the word
    /// * `dir` - Direction to place the word (HORIZONTAL or VERTICAL)
    /// * `word` - The word to place (as a string)
    ///
    /// # Panics
    ///
    /// Panics if a position is already occupied by a different letter.
    pub fn play_word(&mut self, pos: &Pos, dir: Direction, word: &str) {
        let mut pos = *pos;
        for ch in word.chars() {
            let tile = ch.into();
            let previous_letter = self.get(&pos).tile;
            if previous_letter.is_none() {
                self.set(&pos, tile);
            } else {
                assert!(previous_letter == Some(tile));
            }
            pos += dir;
        }
    }

    /// Gets the base point value for a tile.
    ///
    /// Returns 0 for wildcard tiles, or the letter's point value from the letter_points map.
    pub fn get_letter_points(&self, tile: Tile) -> i32 {
        match tile {
            Tile::Letter(ch) => self.letter_points.get(&ch).cloned().unwrap_or(0),
            Tile::Wildcard(_) => 0,
        }
    }

    /// Calculates the total score for playing a word at the given position.
    ///
    /// This includes the main word score plus any crossing words formed perpendicular to it.
    /// Takes into account letter multipliers, word multipliers, and bonus squares.
    ///
    /// # Arguments
    ///
    /// * `word_iter` - Iterator over the tiles to play
    /// * `pos` - Starting position for the word
    /// * `dir` - Direction to place the word (HORIZONTAL or VERTICAL)
    ///
    /// # Returns
    ///
    /// The total score including the main word and all crossing words.
    pub fn calc_word_points<I: Iterator<Item = Tile> + Clone>(
        &self,
        word_iter: &mut I,
        pos: &Pos,
        dir: Direction,
    ) -> i32 {
        let mut pos = *pos;
        let mut points = self.calc_word_points_straight(&mut word_iter.clone(), &pos, dir);
        for ch in word_iter {
            if self.is_free(&pos) {
                if let Some((cx, cy, crossing_word)) =
                    self.get_surrounding_letters_pos(&pos, dir.flip())
                {
                    // Create an iterator over the crossing word's letters with the empty spot
                    // replaced with ch.
                    let mut word_iter = crossing_word.map(|bsq| bsq.tile.unwrap_or(ch));
                    points += self.calc_word_points_straight(
                        &mut word_iter,
                        &Pos::new(cx, cy),
                        dir.flip(),
                    );
                }
            }
            pos += dir;
        }
        points
    }

    fn calc_word_points_straight<I: Iterator<Item = Tile>>(
        &self,
        word_iter: &mut I,
        pos: &Pos,
        dir: Direction,
    ) -> i32 {
        let mut pos = *pos;
        let mut points = 0;
        let mut word_mult = 1;
        let mut word_points = 0;
        let mut num_tiles = 0;
        for ch in word_iter {
            let mut ch_points = self.get_letter_points(ch);
            if self.is_free(&pos) {
                let square = self.get(&pos);
                ch_points *= square.letter_bonus;
                word_mult *= square.word_bonus;
                num_tiles += 1;
            } else {
                debug_assert_eq!(self.get(&pos).tile, Some(ch));
            }

            word_points += ch_points;
            pos += dir;
        }

        points += word_points * word_mult;

        if num_tiles >= 7 {
            points += 40;
        }

        points
    }
}

#[derive(Debug, PartialEq)]
enum ParseError {
    Length { pattern: String },
    BonusDigit { digit: char, pattern: String },
    BonusType { bonus: char, pattern: String },
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Length { pattern } => {
                write!(f, "Expected pattern with length 2, found '{pattern}'")
            }
            ParseError::BonusDigit { digit, pattern } => {
                write!(
                    f,
                    "Expected first character of pattern '{pattern}' to be a digit but found '{digit}'",
                )
            }
            ParseError::BonusType { bonus, pattern } => write!(
                f,
                "Expected bonus type of pattern '{pattern}' to be either 'w' or 'l' but found '{bonus}'"
            ),
        }
    }
}

fn parse_board_line(s: &str) -> Result<Vec<BoardSquare>, ParseError> {
    s.split(' ').map(parse_board_square).collect()
}

/// Parses a string describing a square bonus, "--", "3l", "2w" etc.
/// # Arguments
/// * `s` - A two character string
fn parse_board_square(s: &str) -> Result<BoardSquare, ParseError> {
    if s == "--" {
        return Ok(BoardSquare {
            letter_bonus: 1,
            word_bonus: 1,
            tile: None,
            connected: false,
        });
    }

    if s.chars().take(3).count() != 2 {
        return Err(ParseError::Length { pattern: s.into() });
    }

    let mut iter = s.chars();
    let count_ch = iter.next().unwrap();
    let bonus = iter.next().unwrap();
    let count = count_ch.to_digit(10).ok_or(ParseError::BonusDigit {
        digit: count_ch,
        pattern: s.into(),
    })? as i32;

    match bonus {
        'w' => Ok(BoardSquare {
            letter_bonus: 1,
            word_bonus: count,
            tile: None,
            connected: false,
        }),
        'l' => Ok(BoardSquare {
            letter_bonus: count,
            word_bonus: 1,
            tile: None,
            connected: false,
        }),
        _ => Err(ParseError::BonusType {
            bonus,
            pattern: s.into(),
        }),
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..15 {
            writeln!(
                f,
                "{}",
                (0..15)
                    .map(|x| self.get(&Pos::new(x, y)).to_char())
                    .join(" ")
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use itertools::iproduct;
    use std::collections::BTreeSet;

    #[test]
    fn parse_board_square_empty() {
        assert_eq!(
            parse_board_square("--"),
            Ok(BoardSquare {
                tile: None,
                letter_bonus: 1,
                word_bonus: 1,
                connected: false
            })
        );
    }

    #[test]
    fn parse_board_square_word_bonus() {
        assert_eq!(
            parse_board_square("3w"),
            Ok(BoardSquare {
                tile: None,
                letter_bonus: 1,
                word_bonus: 3,
                connected: false
            })
        );
    }

    #[test]
    fn parse_board_square_letter_bonus() {
        assert_eq!(
            parse_board_square("2l"),
            Ok(BoardSquare {
                tile: None,
                letter_bonus: 2,
                word_bonus: 1,
                connected: false
            })
        );
    }

    #[test]
    fn parse_board_square_error_bonustype() {
        for &illegal_pattern in &["2-", "3 ", "12"] {
            assert_eq!(
                parse_board_square(illegal_pattern),
                Err(ParseError::BonusType {
                    bonus: illegal_pattern.chars().nth(1).unwrap(),
                    pattern: illegal_pattern.into(),
                }),
                "{}",
                illegal_pattern
            );
        }
    }

    #[test]
    fn parse_board_square_error_bonusdigit() {
        for &illegal_pattern in &["-1", "ww", "lw", " w"] {
            assert_eq!(
                parse_board_square(illegal_pattern),
                Err(ParseError::BonusDigit {
                    digit: illegal_pattern.chars().next().unwrap(),
                    pattern: illegal_pattern.into(),
                }),
                "{}",
                illegal_pattern
            );
        }
    }

    #[test]
    fn parse_board_square_error_bonustype_multibyte() {
        // Just make sure that these don't give length errors.
        for &illegal_pattern in &["2å", "3字"] {
            assert_eq!(
                parse_board_square(illegal_pattern),
                Err(ParseError::BonusType {
                    bonus: illegal_pattern.chars().nth(1).unwrap(),
                    pattern: illegal_pattern.into(),
                }),
                "{}",
                illegal_pattern
            );
        }
    }

    #[test]
    fn parse_board_square_error_bonusdigit_multibyte() {
        // Just make sure that these don't give length errors.
        for &illegal_pattern in &["ål", "字w"] {
            assert_eq!(
                parse_board_square(illegal_pattern),
                Err(ParseError::BonusDigit {
                    digit: illegal_pattern.chars().next().unwrap(),
                    pattern: illegal_pattern.into(),
                }),
                "{}",
                illegal_pattern
            );
        }
    }

    #[test]
    fn parse_board_square_error_length() {
        for &illegal_pattern in &["---", "w", "l2 ", " --", "", "l", "10l", "11w"] {
            assert_eq!(
                parse_board_square(illegal_pattern),
                Err(ParseError::Length {
                    pattern: illegal_pattern.into()
                }),
                "{}",
                illegal_pattern
            );
        }
    }

    fn parse_to_char(s: &str) -> char {
        parse_board_square(s).unwrap().to_char()
    }

    #[test]
    fn to_char_board_square_empty() {
        assert_eq!(parse_to_char("--"), '·');
    }

    #[test]
    fn to_char_board_square_word_bonus_2() {
        assert_eq!(parse_to_char("2w"), '₂');
    }

    #[test]
    fn to_char_board_square_word_bonus_3() {
        assert_eq!(parse_to_char("3w"), '₃');
    }

    #[test]
    fn to_char_board_square_letter_bonus_2() {
        assert_eq!(parse_to_char("2l"), '²');
    }

    #[test]
    fn to_char_board_square_letter_bonus_3() {
        assert_eq!(parse_to_char("3l"), '³');
    }

    fn parse_display(s: &str) -> String {
        parse_board_square(s).unwrap().to_string()
    }

    #[test]
    fn display_board_square_empty() {
        assert_eq!(parse_display("--"), "--");
    }

    #[test]
    fn display_board_square_word_bonus_2() {
        assert_eq!(parse_display("2w"), "2w");
    }

    #[test]
    fn display_board_square_word_bonus_3() {
        assert_eq!(parse_display("3w"), "3w");
    }

    #[test]
    fn display_board_square_letter_bonus_2() {
        assert_eq!(parse_display("2l"), "2l");
    }

    #[test]
    fn display_board_square_letter_bonus_3() {
        assert_eq!(parse_display("3l"), "3l");
    }

    #[test]
    fn surrounding_letters() {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        let valid76h = board.get_surrounding_letters(&Pos::new(7, 6), HORIZONTAL);
        assert!(valid76h.is_none());

        let valid76v: String = board
            .get_surrounding_letters(&Pos::new(7, 6), VERTICAL)
            .unwrap()
            .map(|bs| bs.tile.map(|tile| tile.into()).unwrap_or(' '))
            .collect();
        assert_eq!(valid76v, " BLOMKÅL");
    }

    #[test]
    fn surrounding_letter_pos_start() {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        let slp = board
            .get_surrounding_letters_pos(&Pos::new(7, 7), VERTICAL)
            .unwrap();
        assert_eq!((slp.0, slp.1), (7, 7));
    }

    #[test]
    fn surrounding_letter_pos_middle() {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        let slp = board
            .get_surrounding_letters_pos(&Pos::new(7, 10), VERTICAL)
            .unwrap();
        assert_eq!((slp.0, slp.1), (7, 7));
    }

    #[test]
    fn surrounding_letter_pos_end() {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        let slp = board
            .get_surrounding_letters_pos(&Pos::new(7, 13), VERTICAL)
            .unwrap();
        assert_eq!((slp.0, slp.1), (7, 7));
    }

    #[test]
    fn surrounding_letters_after() {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        let valid: String = board
            .get_surrounding_letters(&Pos::new(8, 7), HORIZONTAL)
            .unwrap()
            .map(|bs| bs.tile.map(|tile| tile.into()).unwrap_or(' '))
            .collect();
        assert_eq!(valid, "B ");

        let valid76v: String = board
            .get_surrounding_letters(&Pos::new(7, 6), VERTICAL)
            .unwrap()
            .map(|bs| bs.tile.map(|tile| tile.into()).unwrap_or(' '))
            .collect();
        assert_eq!(valid76v, " BLOMKÅL");
    }

    #[test]
    fn score() {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        let score = board.calc_word_points(
            &mut "SLÅTTER".chars().map(|ch| ch.into()),
            &Pos::new(7, 14),
            HORIZONTAL,
        );
        assert_eq!(score, 97);
    }

    #[test]
    fn score_straight() {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        let score = board.calc_word_points_straight(
            &mut "SLÅTTER".chars().map(|ch| ch.into()),
            &Pos::new(7, 14),
            HORIZONTAL,
        );
        assert_eq!(score, 76);
    }

    #[test]
    fn direction_horizontal() {
        let dir = HORIZONTAL;

        assert_eq!(
            dir.flip(),
            VERTICAL,
            "Flipping horizontal returns a vertical"
        );

        assert_eq!(dir, HORIZONTAL, "Original dir isn't changed by flip()");

        assert_eq!(
            dir.flip().flip(),
            HORIZONTAL,
            "Flipping horizontal twice returns horizontal"
        );
    }

    #[test]
    fn direction_vertical() {
        let dir = VERTICAL;

        assert_eq!(
            dir.flip(),
            HORIZONTAL,
            "Flipping vertical returns a horizontal"
        );

        assert_eq!(dir, VERTICAL, "Original dir isn't changed by flip()");

        assert_eq!(
            dir.flip().flip(),
            VERTICAL,
            "Flipping vertical twice returns vertical"
        );
    }

    #[test]
    fn within_bounds() {
        for (x, y) in iproduct!(-10..25, -10..25) {
            assert_eq!(
                Board::within_bounds(&Pos::new(x, y)),
                (0..15).contains(&x) && (0..15).contains(&y)
            );
        }
    }

    #[test]
    fn is_connected() {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        board.play_word(&Pos::new(7, 10), HORIZONTAL, "MAKALÖS");

        let neighbours: BTreeSet<_> = [(7, 6), (7, 14), (14, 10)]
            .iter()
            .cloned()
            .chain((7..14).map(|y| (6, y))) // Left of BLOMKÅL.
            .chain((7..14).map(|y| (8, y))) // Right of BLOMKÅL
            .chain((8..14).map(|x| (x, 9))) // Above MAKALÖS.
            .chain((8..14).map(|x| (x, 11))) // Below MAKALÖS.
            .collect();

        for (x, y) in iproduct!(0..15, 0..15) {
            let pos = Pos::new(x, y);
            if board.is_free(&pos) {
                assert_eq!(
                    board.is_connected(&pos),
                    neighbours.contains(&(x, y)),
                    "x:{}, y:{}",
                    x,
                    y,
                );
            }
        }
    }

    #[test]
    fn pos_addition_horizontal() {
        let pos = Pos::new(1, 5);
        let dir = HORIZONTAL;
        assert_eq!(pos + dir, Pos::new(2, 5));
    }

    #[test]
    fn pos_addition_vertical() {
        let pos = Pos::new(1, 5);
        let dir = VERTICAL;
        assert_eq!(pos + dir, Pos::new(1, 6));
    }

    #[test]
    fn pos_addition_horizontal_vertical() {
        let pos = Pos::new(6, 7);
        assert_eq!(pos + HORIZONTAL + VERTICAL, Pos::new(7, 8));
    }

    #[test]
    fn pos_addition_horizontal_vertical_multiplication() {
        let pos = Pos::new(6, 7);
        assert_eq!(
            pos + HORIZONTAL * 7 + VERTICAL * -1 + HORIZONTAL * 2,
            Pos::new(15, 6)
        );
    }

    #[test]
    fn pos_addition_negative_horizontal() {
        let pos = Pos::new(1, 5);
        let dir = -HORIZONTAL;
        assert_eq!(pos + dir, Pos::new(0, 5));
    }

    #[test]
    fn pos_addition_negative_vertical() {
        let pos = Pos::new(1, 5);
        let dir = -VERTICAL;
        assert_eq!(pos + dir, Pos::new(1, 4));
    }

    #[test]
    fn dir_multiplication_horizontal() {
        let dir = HORIZONTAL;
        assert_eq!(dir * 8, Direction { dx: 8, dy: 0 })
    }

    #[test]
    fn dir_multiplication_vertical() {
        let dir = VERTICAL;
        assert_eq!(dir * 8, Direction { dx: 0, dy: 8 })
    }

    #[test]
    fn pos_addition_dir_multiplication_horizontal() {
        let pos = Pos::new(1, 5);
        let dir = HORIZONTAL;
        assert_eq!(pos + dir * 8, Pos::new(9, 5))
    }

    #[test]
    fn pos_addition_dir_multiplication_vertical() {
        let pos = Pos::new(1, 5);
        let dir = VERTICAL;
        assert_eq!(pos + dir * 8, Pos::new(1, 13))
    }

    #[test]
    fn i32_to_char() {
        let expected = [
            (0, '0'),
            (1, '1'),
            (2, '2'),
            (3, '3'),
            (4, '4'),
            (5, '5'),
            (6, '6'),
            (7, '7'),
            (8, '8'),
            (9, '9'),
        ];
        for (i, ch) in expected {
            assert_eq!(BoardSquare::i32_to_char(i), ch);
        }
    }

    #[test]
    #[should_panic]
    fn i32_to_char_too_big() {
        BoardSquare::i32_to_char(10);
    }

    #[test]
    #[should_panic]
    fn i32_to_char_negative() {
        BoardSquare::i32_to_char(-1);
    }
}
