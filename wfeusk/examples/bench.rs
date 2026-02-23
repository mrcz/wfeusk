use libdawg::dawg::{children::Children, DawgNode};
use std::time::Duration;
use std::{mem, thread};
use typed_arena::Arena;
use wfeusk::board::{Board, Pos, HORIZONTAL, VERTICAL};
use wfeusk::matcher;
use wfeusk::wordlist::{build_wordlist_from_file, Wordlist};

/// Benchmark example that replicates the old main.rs behavior.
///
/// This example is designed for performance benchmarking and PGO (Profile-Guided Optimization).
/// It uses hardcoded values (board with "BLOMKÅL", rack "PASSARE") to ensure consistent
/// benchmark results that are comparable with older versions of the code.
///
/// Run with: cargo run --release --example bench
fn main() {
    let arena = Arena::new();

    // Use default arguments that match the old behavior
    let wordlist_file = "dict-sv.txt";
    let letters = "PASSARE";

    {
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");

        let _score = board.calc_word_points(
            &mut "SLÅTTER".chars().map(|ch| ch.into()),
            &Pos::new(7, 14),
            HORIZONTAL,
        );
        let wordlist = Wordlist::new(build_wordlist_from_file(&arena, wordlist_file).unwrap());

        // Run the word matching algorithm
        let _matches = matcher::find_all_words(&board, &wordlist, letters);

        // Optionally print stats if BENCH_STATS env var is set
        if std::env::var("BENCH_STATS").is_ok() {
            let stats = [
                (
                    "DawgNode size",
                    mem::size_of::<DawgNode<'static, char>>(),
                ),
                ("Children size", mem::size_of::<Children<'static, char>>()),
                ("Node count", arena.len()),
            ];
            for (label, value) in &stats {
                println!("{label}: {value}");
            }
        }
    }

    // Sleep if BENCH_SLEEP env var is set (useful for profiling)
    if let Ok(sleep_str) = std::env::var("BENCH_SLEEP") {
        if let Ok(sleep) = sleep_str.parse::<u64>() {
            thread::sleep(Duration::from_millis(sleep));
        }
    }
}
