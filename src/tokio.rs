use crate::CommandResponse;
use crate::client::Client;
use crate::commands::Command;
use crate::frame::{self, DecodeError, Request};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

impl<T: AsyncRead + AsyncWrite + Unpin> Client<T> {
    pub async fn send_command_async(
        &mut self,
        command: Command,
    ) -> Result<Vec<CommandResponse>, DecodeError> {
        let command_identifier = command.identifier();
        let request = Request::new(command);
        let encoded_request = frame::encode(request);
        log::debug!("TX: {:02X?}", encoded_request);
        self.transport.write_all(&encoded_request).await?;

        let mut started = false;
        loop {
            let mut buf = [0; 16];
            let res = self.transport.read(&mut buf).await?;

            if res == 0 {
                if !started {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "No response received",
                    )
                    .into());
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Partial response received",
                    )
                    .into());
                }
            }
            started = true;
            log::debug!("RX: {:02X?}", &buf[..res]);
            let responses = self.decoder.feed(&buf[..res])?;
            let responses: Vec<_> = responses
                .into_iter()
                .flat_map(|r| r.data)
                .filter(|r| r.identifier() == command_identifier)
                .collect();
            if responses.is_empty() {
                continue;
            }
            return Ok(responses);
        }
    }
}
