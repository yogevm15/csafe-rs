use crate::frame::DecodeError;

pub enum Command {}

pub enum CommandResponse {}

impl Command {
    pub fn identifier(&self) -> u8 {
        match self {}
    }

    pub fn data(&self) -> Option<Vec<u8>> {
        match self {}
    }

    pub fn from_identifier_and_data(id: u8, data: Option<&[u8]>) -> Result<Self, DecodeError> {
        match id {
            _ => Err(DecodeError::UnknownCommand),
        }
    }
}

impl CommandResponse {
    pub fn identifier(&self) -> u8 {
        match self {}
    }

    pub fn data(&self) -> Vec<u8> {
        match self {}
    }

    pub fn from_identifier_and_data(id: u8, data: &[u8]) -> Result<Self, DecodeError> {
        match id {
            _ => Err(DecodeError::UnknownCommand),
        }
    }
}
