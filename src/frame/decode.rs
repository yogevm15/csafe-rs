use crate::frame::byte_stuff::unstuff;
use crate::frame::checksum::{ChecksumDecoder, InvalidChecksum};
use crate::frame::{END_FLAG, START_FLAG};

pub enum FrameDecodeError {
    InvalidChecksum,
    InvalidStartFlag,
    InvalidEndFlag,
}

impl From<FrameDecodeError> for std::io::Error {
    fn from(e: FrameDecodeError) -> Self {
        let msg = match e {
            FrameDecodeError::InvalidChecksum => "Invalid checksum",
            FrameDecodeError::InvalidStartFlag => "Invalid start flag",
            FrameDecodeError::InvalidEndFlag => "Invalid end flag",
        };

        std::io::Error::new(std::io::ErrorKind::InvalidData, msg)
    }
}

pub trait Decoder: Sized {
    type Output;
    fn feed(self, data: &[u8]) -> Result<(Self::Output, usize), Self>;
}

pub enum FrameDecoder<D: Decoder> {
    WaitingForStart { decoder: ChecksumDecoder<D> },
    WaitingForData { decoder: ChecksumDecoder<D> },
    WaitingForEnd { output: D::Output },
}

impl<D: Decoder> FrameDecoder<D> {
    pub fn new(decoder: D) -> Self {
        FrameDecoder::WaitingForStart {
            decoder: ChecksumDecoder::new(decoder),
        }
    }
}

impl<D: Decoder> Decoder for FrameDecoder<D> {
    type Output = Result<D::Output, FrameDecodeError>;

    fn feed(self, data: &[u8]) -> Result<(Self::Output, usize), Self> {
        match self {
            FrameDecoder::WaitingForStart { decoder } => {
                if data.is_empty() {
                    return Err(FrameDecoder::WaitingForStart { decoder });
                }

                if data[0] != START_FLAG {
                    return Ok((Err(FrameDecodeError::InvalidStartFlag), 0));
                }

                FrameDecoder::WaitingForData { decoder }
                    .feed(&data[1..])
                    .map(|(o, n)| (o, n + 1))
            }
            FrameDecoder::WaitingForData { mut decoder } => {
                let mut total = 0;
                for data in unstuff(data) {
                    match decoder.feed(data) {
                        Ok((Ok(output), n)) => {
                            total += n;
                            let (_, remaining) = data.split_at(n);

                            return FrameDecoder::WaitingForEnd { output }
                                .feed(remaining)
                                .map(|(o, n)| (o, total + n));
                        }
                        Ok((Err(InvalidChecksum), n)) => {
                            return Ok((Err(FrameDecodeError::InvalidChecksum), total + n));
                        }
                        Err(d) => decoder = d,
                    }

                    total += data.len();
                }

                Err(FrameDecoder::WaitingForData { decoder })
            }
            FrameDecoder::WaitingForEnd { output } => {
                if data.is_empty() {
                    return Err(FrameDecoder::WaitingForEnd { output });
                }
                if data[0] != END_FLAG {
                    return Ok((Err(FrameDecodeError::InvalidEndFlag), 0));
                }
                Ok((Ok(output), 1))
            }
        }
    }
}
