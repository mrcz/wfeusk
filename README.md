# wfeusk

A Scrabble/word game solver written in Rust. Given a board state and a rack of letters, finds all valid word placements.

Uses a [DAWG](https://en.wikipedia.org/wiki/Deterministic_acyclic_finite_state_automaton) (Directed Acyclic Word Graph) via [libdawg](https://crates.io/crates/libdawg) for efficient word lookup. The core algorithm is based on [Daciuk et al. (2000)](https://arxiv.org/abs/cs/0007009v1).

## Building

```bash
cargo build --release
```

## Usage

```bash
# Find words with a rack of letters (Swedish dictionary, default)
cargo run --release -- -r PASSARE

# Use the English dictionary
cargo run --release -- -w dict-en.txt -r EXAMPLE

# Show DAWG statistics
cargo run --release -- -s

# Debug output (grouped by position and direction)
cargo run --release -- -r PASSARE -d
```

### CLI flags

| Flag | Long | Description |
|------|------|-------------|
| `-w` | `--wordlist` | Dictionary file to load (default: `dict-sv.txt`) |
| `-r` | `--rack` | Letters in your rack |
| `-s` | `--stats` | Print DAWG statistics |
| `-d` | `--debug` | Print debug info |
| `-z` | `--sleep` | Sleep N ms after loading (for profiling) |

## Benchmarking

```bash
# Criterion benchmarks
cargo bench

# PGO-optimized benchmarks
./pgo-bench.sh
```

## License

MIT
