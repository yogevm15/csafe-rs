use std::slice;
use crate::frame::byte_stuff::stuff;
use crate::frame::checksum::checksum_iter;
use crate::frame::{END_FLAG, START_FLAG};


pub trait Encode {
    fn encode(self) -> Vec<u8>;
}

pub fn encode(data: &impl Encode) -> impl Iterator<Item = &[u8]> {
    std::iter::once(&[START_FLAG][..])
        .chain(checksum_iter(data.encode()).flat_map(stuff))
        .chain(std::iter::once(&[END_FLAG][..]))
}


/// Promote a byte to a static slice.
pub fn promote(byte: u8) -> &'static [u8] {
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
