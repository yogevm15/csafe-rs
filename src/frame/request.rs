use crate::Command;
use crate::frame::decode::DecodeError;
use crate::frame::{Decode, Encode};

pub struct Request {
    command: Command
}

impl Request {
    pub fn new(command: Command) -> Self {
        Self {
            command
        }
    }
}

impl Encode for Request {
    fn encode(self) -> Vec<u8> {
        let data = self.command.data();
        [self.command.identifier()]
            .into_iter()
            .chain(data.as_ref().map(|d| d.len() as u8).into_iter())
            .chain(data.unwrap_or_default())
            .collect()
    }
}

impl Decode for Request {
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.is_empty() {
            return Err(DecodeError::UnexpectedEndOfData);
        }

        let identifier = data[0];

        if data.len() < 2 {
            return Command::from_identifier_and_data(identifier, None).map(Self::new);
        }
        let data_len = data[1] as usize;
        if data.len() < 2 + data_len {
            return Err(DecodeError::UnexpectedEndOfData);
        }

        Command::from_identifier_and_data(identifier, Some(&data[2..2 + data_len])).map(Self::new)
    }
}
