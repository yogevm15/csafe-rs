use crate::frame::Decoder;

pub trait Command {
    type Response;
    /// The CSAFE command identifier byte (e.g. 0xA5 for GetSpeed).
    fn id() -> u8;
    fn encode(&self) -> impl Iterator<Item = &[u8]>;
    fn response_decoder() -> impl Decoder<Output = Self::Response>;
}

// ── Unit helpers ─────────────────────────────────────────────────────────────

/// CSAFE unit codes relevant to workout data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    MilePerHour,
    TenthMilePerHour,
    HundredthMilePerHour,
    KmPerHour,
    TenthKmPerHour,
    HundredthKmPerHour,
    PercentGrade,
    HundredthPercentGrade,
    TenthPercentGrade,
    Unknown(u8),
}

impl Unit {
    pub fn from_byte(b: u8) -> Self {
        match b {
            16 => Unit::MilePerHour,
            17 => Unit::TenthMilePerHour,
            18 => Unit::HundredthMilePerHour,
            48 => Unit::KmPerHour,
            49 => Unit::TenthKmPerHour,
            50 => Unit::HundredthKmPerHour,
            74 => Unit::PercentGrade,
            75 => Unit::HundredthPercentGrade,
            76 => Unit::TenthPercentGrade,
            other => Unit::Unknown(other),
        }
    }

    /// Returns the display suffix and scale divisor for the raw integer value.
    /// `(suffix, divisor)` — divide raw value by divisor to get the human number.
    pub fn display_info(&self) -> (&'static str, f64) {
        match self {
            Unit::MilePerHour => ("mph", 1.0),
            Unit::TenthMilePerHour => ("mph", 10.0),
            Unit::HundredthMilePerHour => ("mph", 100.0),
            Unit::KmPerHour => ("km/h", 1.0),
            Unit::TenthKmPerHour => ("km/h", 10.0),
            Unit::HundredthKmPerHour => ("km/h", 100.0),
            Unit::PercentGrade => ("% grade", 1.0),
            Unit::HundredthPercentGrade => ("% grade", 100.0),
            Unit::TenthPercentGrade => ("% grade", 10.0),
            Unit::Unknown(_) => ("(unknown unit)", 1.0),
        }
    }
}

// ── Response types ────────────────────────────────────────────────────────────

/// Response for commands returning an integer + unit specifier (3 bytes total).
#[derive(Debug, Clone, Copy)]
pub struct ValueWithUnit {
    /// Raw 16-bit little-endian value (signed to support negative grade/incline).
    pub raw: i16,
    /// Parsed unit.
    pub unit: Unit,
}

impl ValueWithUnit {
    /// Returns the human-readable (scaled) float value and its unit suffix.
    pub fn display(&self) -> (f64, &'static str) {
        let (suffix, divisor) = self.unit.display_info();
        (self.raw as f64 / divisor, suffix)
    }
}

/// Response for `GetTWork` — workout duration in H:M:S.
#[derive(Debug, Clone, Copy)]
pub struct WorkoutTime {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

// ── Data-only decoders ────────────────────────────────────────────────────────

pub struct ValueWithUnitDecoder {
    buf: [u8; 3],
    filled: usize,
}

impl ValueWithUnitDecoder {
    pub fn new() -> Self {
        Self {
            buf: [0; 3],
            filled: 0,
        }
    }
}

impl Decoder for ValueWithUnitDecoder {
    type Output = ValueWithUnit;

    fn feed(mut self, data: &[u8]) -> Result<(Self::Output, usize), Self> {
        let need = 3 - self.filled;
        let take = need.min(data.len());
        self.buf[self.filled..self.filled + take].copy_from_slice(&data[..take]);
        self.filled += take;
        if self.filled == 3 {
            let raw = i16::from_le_bytes([self.buf[0], self.buf[1]]);
            let unit = Unit::from_byte(self.buf[2]);
            Ok((ValueWithUnit { raw, unit }, take))
        } else {
            Err(self)
        }
    }
}

pub struct WorkoutTimeDecoder {
    buf: [u8; 3],
    filled: usize,
}

impl WorkoutTimeDecoder {
    pub fn new() -> Self {
        Self {
            buf: [0; 3],
            filled: 0,
        }
    }
}

impl Decoder for WorkoutTimeDecoder {
    type Output = WorkoutTime;

    fn feed(mut self, data: &[u8]) -> Result<(Self::Output, usize), Self> {
        let need = 3 - self.filled;
        let take = need.min(data.len());
        self.buf[self.filled..self.filled + take].copy_from_slice(&data[..take]);
        self.filled += take;
        if self.filled == 3 {
            Ok((
                WorkoutTime {
                    hours: self.buf[0],
                    minutes: self.buf[1],
                    seconds: self.buf[2],
                },
                take,
            ))
        } else {
            Err(self)
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// `cmdGetSpeed` (0xA5) — current treadmill speed.
pub struct GetSpeed;

impl Command for GetSpeed {
    type Response = ValueWithUnit;

    fn id() -> u8 {
        0xA5
    }

    fn encode(&self) -> impl Iterator<Item = &[u8]> {
        std::iter::once(&[0xA5u8][..])
    }

    fn response_decoder() -> impl Decoder<Output = Self::Response> {
        ValueWithUnitDecoder::new()
    }
}

/// `cmdGetGrade` (0xA8) — current incline/grade.
pub struct GetGrade;

impl Command for GetGrade {
    type Response = ValueWithUnit;

    fn id() -> u8 {
        0xA8
    }

    fn encode(&self) -> impl Iterator<Item = &[u8]> {
        std::iter::once(&[0xA8u8][..])
    }

    fn response_decoder() -> impl Decoder<Output = Self::Response> {
        ValueWithUnitDecoder::new()
    }
}

/// `cmdGetTWork` (0xA0) — elapsed workout duration (H:M:S).
pub struct GetTWork;

impl Command for GetTWork {
    type Response = WorkoutTime;

    fn id() -> u8 {
        0xA0
    }

    fn encode(&self) -> impl Iterator<Item = &[u8]> {
        std::iter::once(&[0xA0u8][..])
    }

    fn response_decoder() -> impl Decoder<Output = Self::Response> {
        WorkoutTimeDecoder::new()
    }
}
