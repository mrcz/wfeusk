use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter;

use libdawg::dawg::builder::Builder;
use libdawg::dawg::DawgNode;
use typed_arena::Arena;

use crate::letters::Letters;

/// Maximum word length for Scrabble (board width).
const MAX_WORD_LEN: usize = 15;
/// Minimum word length for Scrabble.
const MIN_WORD_LEN: usize = 2;

/// Builds a DAWG from a dictionary file, filtering to valid Scrabble word lengths (2–15).
///
/// Lines starting with '#' are treated as comments and skipped.
pub fn build_wordlist_from_file<'arena>(
    arena: &'arena Arena<DawgNode<'arena, char>>,
    filename: &str,
) -> Result<&'arena DawgNode<'arena, char>, Box<dyn Error>> {
    let mut builder = Builder::new(arena);
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut buf = String::with_capacity(80);
    loop {
        let bytes_read = reader.read_line(&mut buf);
        match bytes_read {
            Ok(0) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        let word = buf.trim_end();
        let len = word.chars().count();
        if (MIN_WORD_LEN..=MAX_WORD_LEN).contains(&len)
            && !word.trim_start().starts_with('#')
        {
            builder.add_word(word)?;
        }
        buf.clear();
    }
    Ok(builder.build())
}

/// A memory-efficient dictionary implemented as a Directed Acyclic Word Graph (DAWG).
///
/// Wraps a root [`DawgNode`] and provides convenient word lookup and query methods.
///
/// # Thread Safety
///
/// `Wordlist` is `Send` and `Sync` because it only contains immutable references to
/// arena-allocated nodes.
#[derive(Debug)]
pub struct Wordlist<'w> {
    root: &'w DawgNode<'w, char>,
}

// SAFETY: Wordlist only contains immutable references to arena-allocated nodes.
// These references are safe to send and share between threads because:
// 1. The arena lives for the entire lifetime 'w
// 2. All node data is immutable after construction
// 3. No interior mutability is used
unsafe impl Send for Wordlist<'_> {}
unsafe impl Sync for Wordlist<'_> {}

impl<'w> Wordlist<'w> {
    /// Creates a new wordlist from a DAWG root node.
    pub fn new(root: &'w DawgNode<'w, char>) -> Self {
        Wordlist { root }
    }

    /// Returns a reference to the root DawgNode.
    #[inline]
    pub fn get_root(&self) -> &'w DawgNode<'w, char> {
        self.root
    }

    /// Returns true iff word is a valid word according to the wordlist.
    pub fn is_word(&self, word: &str) -> bool {
        self.contains_word(word.chars())
    }

    /// Returns true iff the given sequence forms a valid word according to the wordlist.
    #[inline]
    pub fn contains_word(&self, word: impl IntoIterator<Item = char>) -> bool {
        word.into_iter()
            .try_fold(self.root, |node, ch| node.get(ch))
            .is_some_and(|n| n.is_word())
    }

    /// Returns the branching factor of the graph as a histogram.
    pub fn branching_factor(&self) -> BTreeMap<usize, usize> {
        fn traverse_graph<'w>(
            node: &'w DawgNode<'w, char>,
            counts: &mut BTreeMap<usize, usize>,
            visited: &mut HashSet<&'w DawgNode<'w, char>>,
        ) {
            counts
                .entry(node.child_count())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            for (_, child) in node.children() {
                if visited.insert(child) {
                    traverse_graph(child, counts, visited);
                }
            }
        }
        let mut counts = BTreeMap::new();
        traverse_graph(self.root, &mut counts, &mut HashSet::new());
        counts
    }

    /// Given a word with a gap in it, return an iterator over characters valid for the gap.
    ///
    /// `word_iter` is a sequence of `Option<char>` with exactly one `None` representing the gap.
    /// Returns the characters that can be inserted into the gap that will make
    /// the whole sequence into a valid word. The characters are guaranteed to be sorted.
    pub fn valid_letters_iter<'s, I>(
        &'s self,
        word_iter: I,
    ) -> Option<impl Iterator<Item = char> + 's>
    where
        I: Iterator<Item = Option<char>> + Clone + 's,
    {
        // In debug mode, assert that we have exactly one space in the char sequence.
        debug_assert_eq!(word_iter.clone().filter(|ch| ch.is_none()).count(), 1);

        // Collect prefix characters before the None (space).
        let mut prefix_chars = smallvec::SmallVec::<[char; 15]>::new();
        let mut remaining_iter = word_iter;

        loop {
            if let Some(opt_ch) = remaining_iter.next() {
                if let Some(ch) = opt_ch {
                    prefix_chars.push(ch);
                } else {
                    // Found the None (space), remaining_iter now points after the space.
                    break;
                }
            } else {
                // Iterator exhausted without finding a space - shouldn't happen per debug_assert.
                return None;
            }
        }

        // Follow the wordlist nodes according to the prefix.
        let prefix_node = prefix_chars
            .into_iter()
            .try_fold(self.root, |node, ch| node.get(ch));

        // remaining_iter is now positioned after the space.
        prefix_node.map(move |pn| ValidLetters::new(pn, remaining_iter))
    }

    /// Given a word with a gap in it, return the characters that are valid to put in the gap.
    ///
    /// Behaves like [`valid_letters_iter`](Self::valid_letters_iter) but collects the result
    /// into a `Letters` set.
    pub fn valid_letters<I>(&self, word_iter: I) -> Letters
    where
        I: Iterator<Item = Option<char>> + Clone,
    {
        self.valid_letters_iter(word_iter)
            .map_or(Letters::empty(), |mut letters| Letters::new(&mut letters))
    }
}

/// Iterator over valid letters that can fill a gap in a word pattern.
///
/// Created by [`Wordlist::valid_letters_iter`] to iterate over characters that would
/// make a valid word when inserted into a specific position.
pub struct ValidLetters<'w, I>
where
    I: Iterator<Item = Option<char>> + Clone,
{
    prefix_node: &'w DawgNode<'w, char>,
    word_iter: I,
    taken: usize,
}

impl<'w, I> ValidLetters<'w, I>
where
    I: Iterator<Item = Option<char>> + Clone,
{
    fn new(prefix_node: &'w DawgNode<'w, char>, word_iter: I) -> Self {
        debug_assert!(word_iter.clone().all(|w| w.is_some()));
        ValidLetters {
            prefix_node,
            word_iter,
            taken: 0,
        }
    }
}

impl<'w, I> Iterator for ValidLetters<'w, I>
where
    I: Iterator<Item = Option<char>> + Clone,
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        for (ch, _) in self.prefix_node.children().skip(self.taken) {
            self.taken += 1;
            let mut suffix = iter::once(ch).chain(
                self.word_iter
                    .clone()
                    .map(|opt| opt.expect("Value has already been checked to be Some")),
            );
            if self.prefix_node.has_suffix(&mut suffix) {
                return Some(ch);
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use libdawg::dawg::builder::build_dawg;
    use typed_arena::Arena;

    fn str_to_iter(s: &str) -> impl Iterator<Item = Option<char>> + Clone + '_ {
        s.chars().map(|ch| match ch {
            ' ' => None,
            ch => Some(ch),
        })
    }

    #[test]
    fn valid_chars() {
        let words = ["BESTSTRING", "TESTSPRING", "TESTSTRING"];
        let arena = Arena::new();
        let root = build_dawg(&arena, words).unwrap();
        let wordlist = Wordlist::new(root);

        let valid_chars = |word: &str| wordlist.valid_letters(str_to_iter(word));

        assert_eq!(valid_chars("TESTS RING"), Letters::from("PT"));
        assert_eq!(valid_chars("TEST TRING"), Letters::from("S"));
        assert_eq!(valid_chars(" ESTSTRING"), Letters::from("BT"));
        assert_eq!(valid_chars("TESTSTRIN "), Letters::from("G"));
        assert_eq!(valid_chars("TESTSTRING "), Letters::from(""));
        assert_eq!(valid_chars(" TESTSTRING"), Letters::from(""));
    }

    #[test]
    fn build_wordlist_skips_comments() {
        let dir = std::env::temp_dir().join("wfeusk_test_comments");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dict.txt");
        std::fs::write(&path, "# comment\nAB\nCD\n").unwrap();

        let arena = Arena::new();
        let root = build_wordlist_from_file(&arena, path.to_str().unwrap()).unwrap();
        let wl = Wordlist::new(root);
        assert!(wl.is_word("AB"));
        assert!(wl.is_word("CD"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn build_wordlist_filters_by_length() {
        let dir = std::env::temp_dir().join("wfeusk_test_length");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dict.txt");
        // "A" is too short (1 char), "AB" is valid, 16 chars is too long
        std::fs::write(&path, "A\nAB\nABCDEFGHIJKLMNOP\n").unwrap();

        let arena = Arena::new();
        let root = build_wordlist_from_file(&arena, path.to_str().unwrap()).unwrap();
        let wl = Wordlist::new(root);
        assert!(!wl.is_word("A"));
        assert!(wl.is_word("AB"));
        assert!(!wl.is_word("ABCDEFGHIJKLMNOP"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
