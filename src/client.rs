use crate::commands::Command;
use crate::frame::{self, Decoder, FrameDecoder};
use std::io::{Read, Write};

pub struct Client<T> {
    pub(crate) transport: T,
}

impl<T> Client<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
}

impl<T: Read + Write> Client<T> {
    pub fn send_command<C: Command>(&mut self, command: C) -> Result<C::Response, std::io::Error> {
        for d in frame::encode(command.encode()) {
            self.transport.write_all(d)?;
        }

        let mut started = false;
        let response_decoder = C::response_decoder();
        let mut frame_decoder = FrameDecoder::new(response_decoder);
        loop {
            let mut buf = [0; 16];
            let res = self.transport.read(&mut buf)?;

            if res == 0 {
                if !started {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "No response received",
                    ));
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Partial response received",
                    ));
                }
            }
            started = true;

            match frame_decoder.feed(&buf[..res]) {
                Ok((Ok(o), _)) => return Ok(o),
                Ok((Err(e), _)) => return Err(e.into()),
                Err(d) => frame_decoder = d,
            }
        }
    }
}
