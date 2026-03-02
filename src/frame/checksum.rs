use crate::frame::decode::Decoder;
use std::slice;

pub fn checksum_iter<'a>(
    data: impl IntoIterator<Item = &'a [u8]>,
) -> impl Iterator<Item = &'a [u8]> {
    ChecksumIter::new(data.into_iter())
}

pub struct ChecksumIter<I> {
    checksum: Option<u8>,
    iter: I,
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
        let checksum = self.checksum.take()?;
        let Some(data) = self.iter.next() else {
            return Some(promote(checksum));
        };

        self.checksum = Some(slice_checksum(checksum, data));
        Some(data)
    }
}

pub struct InvalidChecksum;

pub enum ChecksumDecoder<D: Decoder> {
    WaitingForData { checksum: u8, inner: D },
    WaitingForChecksum { checksum: u8, output: D::Output },
}

impl<D: Decoder> ChecksumDecoder<D> {
    pub fn new(decoder: D) -> Self {
        Self::WaitingForData {
            checksum: 0,
            inner: decoder,
        }
    }
}

impl<D: Decoder> Decoder for ChecksumDecoder<D> {
    type Output = Result<D::Output, InvalidChecksum>;

    fn feed(self, data: &[u8]) -> Result<(Self::Output, usize), Self> {
        match self {
            ChecksumDecoder::WaitingForData { checksum, inner } => match inner.feed(data) {
                Ok((output, read)) => {
                    let (read_data, remaining) = data.split_at(read);
                    ChecksumDecoder::WaitingForChecksum {
                        output,
                        checksum: slice_checksum(checksum, read_data),
                    }
                    .feed(remaining)
                    .map(|(o, n)| (o, read + n))
                }
                Err(inner) => Err(ChecksumDecoder::WaitingForData {
                    inner,
                    checksum: slice_checksum(checksum, data),
                }),
            },
            ChecksumDecoder::WaitingForChecksum { output, checksum } => {
                if data.is_empty() {
                    return Err(ChecksumDecoder::WaitingForChecksum { output, checksum });
                }

                if checksum != data[0] {
                    return Ok((Err(InvalidChecksum), 1));
                }

                Ok((Ok(output), 1))
            }
        }
    }
}

fn slice_checksum(checksum: u8, data: &[u8]) -> u8 {
    data.iter().fold(checksum, |acc, &x| acc ^ x)
}

/// Promote a byte to a static slice.
fn promote(byte: u8) -> &'static [u8] {
    // A static array where each element is a single-byte array.
    static BYTE_SLICES: [u8; u8::MAX as usize + 1] = {
        let mut arr = [0; u8::MAX as usize + 1];
        let mut i = 0;
        while i <= u8::MAX as usize {
            arr[i] = i as u8;
            i += 1;
        }
        arr
    };
    // We return a slice to one of the elements of our static array.
    slice::from_ref(&BYTE_SLICES[byte as usize])
}
