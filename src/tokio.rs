use crate::client::Client;
use crate::commands::Command;
use crate::frame;
use crate::frame::{Decoder, FrameDecoder};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

impl<T: AsyncRead + AsyncWrite + Unpin> Client<T> {
    pub async fn send_command_async<C: Command>(
        &mut self,
        command: C,
    ) -> Result<C::Response, std::io::Error> {
        for d in frame::encode(command.encode()) {
            self.transport.write_all(d).await?;
        }

        let mut started = false;
        let response_decoder = C::response_decoder();
        let mut frame_decoder = FrameDecoder::new(response_decoder);
        loop {
            let mut buf = [0; 16];
            let res = self.transport.read(&mut buf).await?;

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
