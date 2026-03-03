use crate::frame::encode::promote;

pub fn checksum_iter(
    data: Vec<u8>,
) -> impl Iterator<Item = &[u8]> {
    ChecksumIter::new(data)
}

pub struct ChecksumIter {
    checksum: Option<u8>,
    iter: Vec<u8>,
}

impl<I> ChecksumIter<I> {
    pub fn new(data: I) -> Self {
        Self {
            checksum: Some(0),
            iter: data,
        }
    }
}

impl<'a, I: Iterator<Item = &'a [u8]>> Iterator for ChecksumIter<I> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let curr_checksum = self.checksum.take()?;
        let Some(data) = self.iter.next() else {
            return Some(promote(curr_checksum));
        };

        self.checksum = Some(checksum(curr_checksum, data));
        Some(data)
    }
}

pub fn checksum(checksum: u8, data: &[u8]) -> u8 {
    data.iter().fold(checksum, |acc, &x| acc ^ x)
}

