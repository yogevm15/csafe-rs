use super::Encode;
use crate::CommandResponse;
use crate::frame::decode::{Decode, DecodeError};
use std::ops::Shl;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum SlaveState {
    /// Serious internal error of type that suggests unit not be used, e.g. unit has lost its calibration parameters. Slave should remain in this State until the problem has been fixed.
    Error = 0,
    /// The Initial state is entered when the Slave is turned on or is reset. The Slave remains in this state until either 1) a user begins a manual workout causing a jump to OffLine State, or 2) the Slave receives configuration commands from the Master and is promoted to Idle State. Once the Idle State is entered, the only way to get back to the Ready State is through a cmdGoReady command.
    Ready = 1,
    /// Slave has been configured by the Master and is now part of the Network environment. The Slave is waiting for a user to enter an ID or a Start event (such as pressing a Start key without entering an ID). This is where a user chooses between 1) a Manual workout that is not monitored by the Master or 2) entering a valid ID to begin a Master sponsored workout.
    Idle = 2,
    /// A user ID or a Start event has been entered. The Master can request the ID and decide based on what was entered whether to issue a command to go the InUse State or back to Idle State.
    HaveID = 3,
    /// The user's workout program is running. If sufficient time elapses without activity or the user presses a pause button, the Pause State is entered. Finishing the workout jumps to the Finished state.
    InUse = 5,
    /// The workout program is halted by the user. If sufficient time passes without the user restarting the program and returning to the InUse state, the Finished state will be entered.
    Paused = 6,
    /// The workout program is completed. The Master solicits the results of the workout and then issues the cmdGoIdle command to return to the Idle State.
    Finished = 7,
    /// The user has elected a workout program that is not supervised by the Master. When he finishes the Slave will automatically return to the Idle state.
    Manual = 8,
    /// The user has begun a workout program and the Master has not configured the Slave. On finishing, the Slave will return to the Ready state.
    Offline = 9,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum PreviousFrameStatus {
    /// Frame was processed without error.
    OK = 0,
    /// Frame was rejected because it was recognized as a legal frame (i.e. not prevBad) but contained a command that was illegal for the current state of the Slave or had illegal syntax.   This also includes commands considered illegal by the Slave – that is, recognized but not permitted.  An unrecognized command with legal syntax is skipped - not rejected,
    Reject = 1,
    /// Frame had bad checksum or overran buffer.  If the Slave can detect it, this status may also be used to indicate missing Start or Stop bytes.
    Bad = 2,
    /// Still processing last frame and cannot receive new frame at this time. NOTE: Many implementations will not be able to respond at all until the previous frame is processed and will simple discard incoming commands until the processing is complete.
    NotReady = 3,
}

pub struct Response {
    slave_state: SlaveState,
    previous_frame_status: PreviousFrameStatus,
    frame_count: bool,
    pub(crate) data: Vec<CommandResponse>,
}

impl Encode for Response {
    fn encode(self) -> Vec<u8> {
        let status = self.slave_state as u8
            | (self.previous_frame_status as u8).shl(4)
            | (self.frame_count as u8).shl(7);
        [status]
            .into_iter()
            .chain(self.data.iter().flat_map(|inner| {
                let data = inner.data();
                [inner.identifier(), data.len() as u8]
                    .into_iter()
                    .chain(data)
            }))
            .collect()
    }
}

impl Decode for Response {
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.is_empty() {
            return Err(DecodeError::UnexpectedEndOfData);
        }

        let status = data[0];
        let slave_state = SlaveState::try_from(status & 0x0F)?;
        let previous_frame_status = PreviousFrameStatus::try_from((status >> 4) & 0x03)?;
        let frame_count = (status >> 7) != 0;

        let mut inner_responses = Vec::new();
        let mut i = 1;
        while i < data.len() {
            let identifier = data[i];
            i += 1;
            if i >= data.len() {
                return Err(DecodeError::UnexpectedEndOfData);
            }

            let len = data[i] as usize;
            if i + len >= data.len() {
                return Err(DecodeError::UnexpectedEndOfData);
            }

            let inner_data = data[i..=i + len].to_vec();
            inner_responses.push(CommandResponse::from_identifier_and_data(
                identifier,
                &inner_data,
            )?);
            i += len + 1;
        }

        Ok(Response {
            slave_state,
            previous_frame_status,
            frame_count,
            data: inner_responses,
        })
    }
}

impl TryFrom<u8> for SlaveState {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, DecodeError> {
        match value {
            0 => Ok(SlaveState::Error),
            1 => Ok(SlaveState::Ready),
            2 => Ok(SlaveState::Idle),
            3 => Ok(SlaveState::HaveID),
            5 => Ok(SlaveState::InUse),
            6 => Ok(SlaveState::Paused),
            7 => Ok(SlaveState::Finished),
            8 => Ok(SlaveState::Manual),
            9 => Ok(SlaveState::Offline),
            _ => Err(DecodeError::InvalidData),
        }
    }
}

impl TryFrom<u8> for PreviousFrameStatus {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, DecodeError> {
        match value {
            0 => Ok(PreviousFrameStatus::OK),
            1 => Ok(PreviousFrameStatus::Reject),
            2 => Ok(PreviousFrameStatus::Bad),
            3 => Ok(PreviousFrameStatus::NotReady),
            _ => Err(DecodeError::InvalidData),
        }
    }
}
