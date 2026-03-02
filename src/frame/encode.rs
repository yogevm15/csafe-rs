use crate::frame::byte_stuff::stuff;
use crate::frame::checksum::checksum_iter;
use crate::frame::{END_FLAG, START_FLAG};

pub fn encode<'a>(data: impl IntoIterator<Item = &'a [u8]>) -> impl Iterator<Item = &'a [u8]> {
    std::iter::once(&[START_FLAG][..])
        .chain(checksum_iter(data).flat_map(stuff))
        .chain(std::iter::once(&[END_FLAG][..]))
}
