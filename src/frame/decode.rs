use crate::frame::byte_stuff::unstuff;
use crate::frame::checksum::checksum;
use crate::frame::{END_FLAG, START_FLAG};
use memchr::memchr;
use std::marker::PhantomData;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Invalid checksum")]
    InvalidChecksum,
    #[error("Unexpected end of data")]
    UnexpectedEndOfData,
    #[error("Invalid start flag")]
    InvalidStartFlag,
    #[error("Invalid data")]
    InvalidData,
    #[error("Unknown command")]
    UnknownCommand,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub trait Decode: Sized {
    fn decode(data: &[u8]) -> Result<Self, DecodeError>;
}

enum FrameDecoderState {
    WaitingForStart,
    WaitingForEnd { data: Vec<u8> },
}

pub struct FrameDecoder<F> {
    state: FrameDecoderState,
    _marker: PhantomData<F>,
}

impl<F> FrameDecoder<F> {
    pub fn new() -> Self {
        Self {
            state: FrameDecoderState::WaitingForStart,
            _marker: PhantomData,
        }
    }
}

impl<F: Decode> FrameDecoder<F> {
    pub fn feed(&mut self, mut data: &[u8]) -> Result<Vec<F>, DecodeError> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        let mut output = vec![];
        loop {
            match &mut self.state {
                FrameDecoderState::WaitingForStart => {
                    if data[0] != START_FLAG {
                        return Err(DecodeError::InvalidStartFlag);
                    }

                    self.state = FrameDecoderState::WaitingForEnd { data: Vec::new() };
                    data = &data[1..];
                    continue;
                }
                FrameDecoderState::WaitingForEnd {
                    data: existing_data,
                } => {
                    let Some(pos) = memchr(END_FLAG, &data) else {
                        for data in unstuff(data) {
                            existing_data.extend_from_slice(data);
                        }
                        return Ok(output);
                    };

                    for data in unstuff(&data[..pos]) {
                        existing_data.extend_from_slice(data);
                    }
                    let (&received_checksum, rest) = existing_data
                        .split_last()
                        .ok_or(DecodeError::UnexpectedEndOfData)?;
                    if received_checksum != checksum(rest) {
                        return Err(DecodeError::InvalidChecksum);
                    }
                    output.push(F::decode(&rest)?);
                    self.state = FrameDecoderState::WaitingForStart;
                    data = &data[pos + 1..];
                    continue;
                }
            }
        }
    }
}
