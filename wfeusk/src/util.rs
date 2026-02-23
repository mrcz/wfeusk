/// Mirror the given iterators elements, for example [1, 2, 3] will return
/// [1, 2, 3, 2, 1] - "mirrored" around the last element (3)
pub fn mirror<'a, T, I>(iter: I) -> impl Iterator<Item = T> + 'a
where
    I: Iterator<Item = T> + DoubleEndedIterator + Clone + 'a,
{
    iter.clone().chain(iter.rev().skip(1))
}

#[cfg(test)]
mod test {
    #[test]
    fn mirror() {
        let mirrored: Vec<_> = super::mirror([1, 2, 3, 4, 5, 6, 7].iter())
            .cloned()
            .collect();
        assert_eq!(mirrored, [1, 2, 3, 4, 5, 6, 7, 6, 5, 4, 3, 2, 1]);

        let mirrored: Vec<_> = super::mirror([1, 2, 1, 99, 1, 1].iter()).cloned().collect();
        assert_eq!(mirrored, [1, 2, 1, 99, 1, 1, 1, 99, 1, 2, 1]);
    }

    #[test]
    fn mirror_empty() {
        let mirrored: Vec<Option<char>> = super::mirror([].iter()).cloned().collect();
        assert_eq!(mirrored, []);
    }
}
