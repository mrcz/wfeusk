use itertools::Itertools;
use libdawg::dawg::{children::Children, DawgNode};
use std::time::Duration;
use std::{mem, thread};
use typed_arena::Arena;
use wfeusk::board::{self, Board, Pos};
use wfeusk::matcher;
use wfeusk::wordlist::{build_wordlist_from_file, Wordlist};

mod arguments;

fn main() {
    let arena = Arena::new();
    let args = arguments::get_arguments();

    // Load wordlist from file with proper error handling
    let wordlist = Wordlist::new(build_wordlist_from_file(&arena, &args.wordlist).unwrap_or_else(
        |e| {
            eprintln!("Error loading wordlist from '{}': {}", args.wordlist, e);
            std::process::exit(1);
        },
    ));

    // If stats mode is requested, display DAWG statistics
    if args.stats {
        let stats = [
            (
                "DawgNode size",
                mem::size_of::<DawgNode<'static, char>>(),
            ),
            ("Children size", mem::size_of::<Children<'static, char>>()),
            ("Node count", arena.len()),
        ];
        println!("DAWG Statistics:");
        for (label, value) in &stats {
            println!("  {label}: {value}");
        }
        println!("\nBranching factor: {:?}", wordlist.branching_factor());
    }

    // If a rack is provided, demonstrate word finding
    if let Some(rack) = args.rack {
        // Create a default board (empty 15x15 grid)
        let board = Board::default();

        // Find all valid word placements with the given rack
        let matches = matcher::find_all_words(&board, &wordlist, &rack);

        println!(
            "Found {} valid word placements with rack '{}'",
            matches.len(),
            rack
        );

        if args.debug {
            // Group matches by position and direction
            let words: Vec<((Pos, board::Direction), Vec<String>)> = matches
                .into_iter()
                .chunk_by(|item| (item.1, item.2))
                .into_iter()
                .map(|(key, group)| (key, group.map(|item| item.0).collect()))
                .collect();
            println!("{:?}", words);
        }
    } else if !args.stats {
        eprintln!("No rack specified. Use -r <LETTERS> to specify a rack of letters.");
        eprintln!("Example: {} -r EXAMPLE", std::env::args().next().unwrap());
        eprintln!("Or use -s to display statistics about the wordlist.");
        std::process::exit(1);
    }

    // Sleep if requested (useful for profiling)
    if let Some(sleep) = args.sleep {
        thread::sleep(Duration::from_millis(sleep));
    }
}
