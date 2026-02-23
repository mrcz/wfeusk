use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use typed_arena::Arena;
use wfeusk::board::{Board, Pos, VERTICAL};
use wfeusk::matcher;
use wfeusk::wordlist::{build_wordlist_from_file, Wordlist};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("load dict-en", |b| {
        b.iter(|| {
            let arena = Arena::new();
            build_wordlist_from_file(&arena, black_box("../dict-en.txt")).unwrap();
        })
    });
    c.bench_function("load dict-se", |b| {
        b.iter(|| {
            let arena = Arena::new();
            build_wordlist_from_file(&arena, black_box("../dict-sv.txt")).unwrap();
        })
    });
    c.bench_function("solve BLOMKÅL/PASSARE", |b| {
        let arena = Arena::new();
        let wordlist =
            Wordlist::new(build_wordlist_from_file(&arena, "../dict-sv.txt").unwrap());
        let mut board = Board::default();
        board.play_word(&Pos::new(7, 7), VERTICAL, "BLOMKÅL");
        b.iter(|| {
            matcher::find_all_words(
                black_box(&board),
                black_box(&wordlist),
                black_box("PASSARE"),
            )
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(20));
    targets = criterion_benchmark
}
criterion_main!(benches);
