use byte_stuff::{F1, F2};
pub use decode::{Decode, FrameDecoder, DecodeError};
pub use encode::{Encode, encode};
pub use request::{Request};
pub use response::{Response};
mod byte_stuff;
mod checksum;
mod decode;
mod encode;
mod response;
mod request;

const START_FLAG: u8 = F1;
const END_FLAG: u8 = F2;
