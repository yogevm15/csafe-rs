use crate::frame::DecodeError;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SpeedUnit {
    KilometerPerHour = 48,
    TenthKilometerPerHour = 49,
    HundredthKilometerPerHour = 50,
    MilePerHour = 16,
    TenthMilePerHour = 17,
    HundredthMilePerHour = 18,
    FeetPerMinute = 19,
    MeterPerMinute = 51,
}

impl std::fmt::Display for SpeedUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KilometerPerHour => write!(f, "km/h"),
            Self::TenthKilometerPerHour => write!(f, "0.1km/h"),
            Self::HundredthKilometerPerHour => write!(f, "0.01km/h"),
            Self::MilePerHour => write!(f, "mi/h"),
            Self::TenthMilePerHour => write!(f, "0.1mi/h"),
            Self::HundredthMilePerHour => write!(f, "0.01mi/h"),
            Self::FeetPerMinute => write!(f, "ft/min"),
            Self::MeterPerMinute => write!(f, "m/min"),
        }
    }
}

impl SpeedUnit {
    pub fn from_u8(value: u8) -> Result<Self, DecodeError> {
        match value {
            48 => Ok(Self::KilometerPerHour),
            49 => Ok(Self::TenthKilometerPerHour),
            50 => Ok(Self::HundredthKilometerPerHour),
            16 => Ok(Self::MilePerHour),
            17 => Ok(Self::TenthMilePerHour),
            18 => Ok(Self::HundredthMilePerHour),
            19 => Ok(Self::FeetPerMinute),
            51 => Ok(Self::MeterPerMinute),
            _ => Err(DecodeError::InvalidData),
        }
    }
}

pub enum Command {
    GetSpeed,
}

pub enum CommandResponse {
    GetSpeed { unit: SpeedUnit, speed: i16 },
}

impl Command {
    pub fn identifier(&self) -> u8 {
        match self {
            Command::GetSpeed => 0xA5,
        }
    }

    pub fn data(&self) -> Option<Vec<u8>> {
        match self {
            Command::GetSpeed => None,
        }
    }

    pub fn from_identifier_and_data(id: u8, data: Option<&[u8]>) -> Result<Self, DecodeError> {
        match id {
            0xA5 => Ok(Command::GetSpeed),
            _ => Err(DecodeError::UnknownCommand),
        }
    }
}

impl CommandResponse {
    pub fn identifier(&self) -> u8 {
        match self {
            CommandResponse::GetSpeed { .. } => 0xA5,
        }
    }

    pub fn data(&self) -> Vec<u8> {
        match self {
            CommandResponse::GetSpeed { unit, speed } => speed
                .to_le_bytes()
                .into_iter()
                .chain([*unit as u8])
                .collect(),
        }
    }

    pub fn from_identifier_and_data(id: u8, data: &[u8]) -> Result<Self, DecodeError> {
        match id {
            0xA5 => {
                let speed = i16::from_le_bytes(data[0..2].try_into().expect("Slice of 2 bytes"));
                let unit = SpeedUnit::from_u8(data[2])?;
                Ok(CommandResponse::GetSpeed { speed, unit })
            }
            _ => Err(DecodeError::UnknownCommand),
        }
    }
}
