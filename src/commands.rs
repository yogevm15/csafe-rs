use crate::frame::Decoder;

pub trait Command {
    type Response;
    fn encode(&self) -> impl Iterator<Item = &[u8]>;
    fn response_decoder() -> impl Decoder<Output = Self::Response>;
}
