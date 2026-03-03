use crate::frame::byte_stuff::stuff;
use crate::frame::checksum::checksum;
use crate::frame::{END_FLAG, START_FLAG};

pub trait Encode {
    fn encode(self) -> Vec<u8>;
}

pub fn encode(data: impl Encode) -> Vec<u8> {
    let data = data.encode();
    let checksum = [checksum(&data)];
    let data = stuff(&data);
    let checksum = stuff(&checksum[..]);
    [START_FLAG]
        .into_iter()
        .chain(data.flatten().copied())
        .chain(checksum.flatten().copied())
        .chain([END_FLAG])
        .collect()
}
