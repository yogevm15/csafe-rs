use byte_stuff::{F1, F2};
pub use decode::{FrameDecoder, Decoder};
pub use encode::encode;
mod byte_stuff;
mod checksum;
mod decode;
mod encode;

const START_FLAG: u8 = F1;
const END_FLAG: u8 = F2;
