use byte_stuff::{F1, F2};
pub use decode::{Decoder, FrameDecoder};
pub use encode::encode;
pub use response::ResponseDecoder;
mod byte_stuff;
mod checksum;
mod decode;
mod encode;
mod response;

const START_FLAG: u8 = F1;
const END_FLAG: u8 = F2;
