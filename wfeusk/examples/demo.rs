use itertools::Itertools;
use libdawg::dawg::builder::build_dawg_from_file;
use typed_arena::Arena;
use wfeusk::board::{self, Board, Pos, HORIZONTAL, VERTICAL};
use wfeusk::matcher;
use wfeusk::wordlist::Wordlist;

/// Demonstrates the wfeusk Scrabble solver with a sample board and rack.
///
/// This example:
/// - Loads the Swedish dictionary (dict-sv.txt)
/// - Creates a board with "BLOMKÅL" placed vertically at the center
/// - Finds all valid words that can be played with the rack "PASSARE"
/// - Demonstrates cross-word validation and scoring
///
/// Run with: cargo run --release --example demo
fn main() {
    let arena = Arena::new();

    // Load the wordlist from file
    let wordlist = Wordlist::new(
        build_dawg_from_file(&arena, "dict-sv.txt")
            .expect("Failed to load wordlist from dict-sv.txt"),
    );

    // Create a new board and place an initial word
    let mut board = Board::default();
    board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");

    // Set up our rack of available letters
    let rack = "PASSARE";

    // Find all valid word placements
    let matches = matcher::find_all_words(&board, &wordlist, rack);

    // Group matches by position and direction
    let grouped_matches: Vec<((Pos, board::Direction), Vec<String>)> = matches
        .into_iter()
        .chunk_by(|item| (item.1, item.2))
        .into_iter()
        .map(|(key, group)| (key, group.map(|item| item.0).collect()))
        .collect();

    // Display the board
    println!("Current board:");
    println!("{}", board);
    println!();

    // Display all found matches
    println!(
        "Found {} valid placements with rack '{}':",
        grouped_matches.len(),
        rack
    );
    for ((pos, dir), words) in &grouped_matches {
        println!("  At {:?} {:?}: {} word(s)", pos, dir, words.len());
        for word in words.iter().take(3) {
            println!("    - {}", word);
        }
        if words.len() > 3 {
            println!("    ... and {} more", words.len() - 3);
        }
    }
    println!();

    // Demonstrate scoring
    let score = board.calc_word_points(
        &mut "SLÅTTER".chars().map(|ch| ch.into()),
        &Pos::new(7, 14),
        HORIZONTAL,
    );
    println!(
        "Example: 'SLÅTTER' at [7,14] HORIZONTAL would score: {} points",
        score
    );

    // Demonstrate cross-word validation
    println!();
    println!("Word validation examples:");
    let test_words = ["HEGEMONIERNA", "PASSARE", "BLOMKÅL", "NOTAWORD"];
    for word in &test_words {
        let valid = wordlist.is_word(word);
        println!(
            "  '{}': {}",
            word,
            if valid { "✓ valid" } else { "✗ invalid" }
        );
    }
}
