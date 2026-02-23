# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**wfeusk** is a Scrabble/word game solver written in Rust. The project consists of two main components:

1. **libdawg** - A library implementing a DAWG (Directed Acyclic Word Graph) data structure for efficient wordlist storage and querying
2. **wfeusk** - The main binary that uses libdawg to find valid word placements on a game board

The core algorithm finds all valid words that can be played on a Scrabble-like board given a rack of letters, using a minimal acyclic finite-state automaton as described in <https://arxiv.org/abs/cs/0007009v1>.

## Build Commands

```bash
# Build the project (default debug build)
cargo build

# Build with optimizations
cargo build --release

# Run the main binary with Swedish dictionary (default)
cargo run --release

# Run with English dictionary
cargo run --release -- -w dict-en.txt

# Run with custom rack of letters
cargo run --release -- -r EXAMPLE

# Show statistics about the DAWG structure
cargo run --release -- -s

# Show debug information
cargo run --release -- -d
```

## Testing

```bash
# Run all tests
cargo test

# Run all tests with verbose output
cargo test -v

# Run tests for a specific package
cargo test -p libdawg
cargo test -p wfeusk

# Run a specific test by name
cargo test <test_name>

# Run tests in the wordlist module
cargo test wordlist

# Common tests to run:
cargo test all_words      # Validates all words in dict-sv.txt
cargo test add_word       # Tests DAWG word insertion
cargo test valid_chars    # Tests character validation
```

## Benchmarking

```bash
# Run benchmarks using criterion
cargo bench

# Simple benchmark script (runs wfeusk 100 times)
./bench.sh

# Profile-guided optimization build
./pgo-build.sh

# Profile-guided optimization benchmark
./pgo-bench.sh
```

## Code Formatting

```bash
# Format code according to rustfmt.toml
cargo fmt

# Check formatting without making changes
cargo fmt -- --check
```

## Architecture

### Workspace Structure

This is a Cargo workspace with two members:

- `wfeusk/` - Main binary crate
- `libdawg/` - Library crate containing the core word-finding logic

### libdawg Library

**Core Components:**

- **`wordlist/`** - DAWG implementation for efficient word storage
  - `dawg/builder.rs` - Builds DAWG from sorted word lists. **CRITICAL**: Words must be added in lexicographic order or building will fail
  - `dawg/children.rs` - `DawgNode` structure representing graph nodes with child edges
  - `wordlist_impl.rs` - `Wordlist` wrapper providing high-level query methods (`is_word()`, `valid_letters()`, etc.)

- **`board.rs`** - Scrabble board representation
  - 15x15 grid with premium squares (double/triple letter/word)
  - `Pos` - Board position coordinates
  - `Direction` - HORIZONTAL or VERTICAL word placement
  - `Board` - Manages tile placement and score calculation

- **`matcher/`** - Core word-finding algorithm
  - `find_all_words()` - Main entry point that finds all valid plays
  - `state.rs` - State machine for traversing DAWG while respecting board constraints
  - Handles both existing tiles on board and new tiles from rack

- **`letters.rs`** - Letter and rack management
  - `Letters` - Set of available letters (bitset representation)
  - `Rack` - Player's tiles including blank handling

- **`tile.rs`** - Individual tile representation with letter and bonus squares

### Key Algorithms

**DAWG Construction** (libdawg/src/wordlist/dawg/builder.rs):

- Words MUST be added in sorted order
- Uses hash-based node deduplication to minimize memory
- Suffix sharing ensures minimal graph size

**Word Finding** (libdawg/src/matcher/):

- Scans each row/column on the board
- For each position, uses DAWG to find valid words using available letters
- Validates cross-words formed perpendicular to the main word
- Returns (word, position, direction) tuples for all valid plays

**Cross-word Validation**:

- When placing a word, perpendicular words must also be valid
- `get_valid_chars()` pre-computes which letters form valid cross-words at each position
- This dramatically reduces the search space

### Memory Management

- Uses `typed_arena::Arena` for DAWG node allocation
- All nodes have lifetime `'arena` tied to the arena
- Enables safe graph traversal without garbage collection overhead
- Arena is allocated once at startup and lives for program duration

## Development Notes

### Working with the DAWG

- The DAWG builder requires words in **strict lexicographic order**
- Use `build_wordlist()` for in-memory word lists or `build_wordlist_from_file()` for dictionary files
- Dictionary files should have one word per line, alphabetically sorted
- Maximum word length is 15 characters (board size constraint)

### Board Coordinate System

- Positions use `Pos::new(x, y)` where both are 0-indexed
- x increases horizontally (left to right)
- y increases vertically (top to bottom)
- Center square is `Pos::new(7, 7)`

### Dictionaries

Two dictionaries are included:

- `dict-sv.txt` - Swedish wordlist (default)
- `dict-en.txt` - English wordlist

### Performance Considerations

- Release builds use `opt-level = 3` and LTO for maximum performance
- Test builds use `opt-level = 1` for faster compilation
- PGO (profile-guided optimization) scripts available for production builds
- The DAWG structure provides O(word length) lookup time
