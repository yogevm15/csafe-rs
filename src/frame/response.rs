use super::Decoder;

// ── CSAFE Response Decoder ────────────────────────────────────────────────────
//
// Per §2.2.2, every response has the structure:
//   Status Byte (1) | Zero or more Data Structures
//
// A Data Structure is: Identifier (1) | Data Byte Count (1) | Data (N)
//
// After reading the status byte the decoder peeks at the next byte:
//   • < 0x80  → cannot be a short-command identifier, so this is a status-only
//               frame (the byte is the checksum). Return None after 1 byte.
//   • == expected_id → our data structure. Read id + count + data, parse via
//               the inner decoder, return Some(parsed).
//   • ≥ 0x80 but ≠ expected_id → a data structure for a *different* command
//               (e.g. a delayed response). Skip id + count + data bytes so the
//               checksum stays aligned, return None.

/// Wraps an inner `Decoder`, handling the CSAFE response envelope.
/// Returns `Option<D::Output>` — `None` for status-only or non-matching frames.
pub struct ResponseDecoder<D: Decoder> {
    expected_id: u8,
    phase: ResponsePhase<D>,
}

#[allow(dead_code)] // inner is carried through skip phases but intentionally unused
enum ResponsePhase<D: Decoder> {
    /// Haven't read anything yet.
    Init(D),
    /// Reading identifier(1) + data byte count(1) for our matching response.
    MatchHeader {
        buf: [u8; 2],
        filled: usize,
        inner: D,
    },
    /// Identifier matched — delegating to inner decoder for data bytes.
    MatchData(D),
    /// Reading identifier(1) + data byte count(1) for a NON-matching response.
    SkipHeader {
        buf: [u8; 2],
        filled: usize,
        inner: D,
    },
    /// Skipping `remaining` data bytes of a non-matching data structure.
    SkipData { remaining: usize, inner: D },
}

impl<D: Decoder> ResponseDecoder<D> {
    pub fn new(expected_id: u8, inner: D) -> Self {
        Self {
            expected_id,
            phase: ResponsePhase::Init(inner),
        }
    }
}

impl<D: Decoder> Decoder for ResponseDecoder<D> {
    type Output = Option<D::Output>;

    fn feed(mut self, data: &[u8]) -> Result<(Self::Output, usize), Self> {
        let mut consumed = 0;

        // ── Phase: read status byte and decide ───────────────────────────
        if let ResponsePhase::Init(inner) = self.phase {
            if data.is_empty() {
                self.phase = ResponsePhase::Init(inner);
                return Err(self);
            }
            // Consume status byte
            consumed += 1;
            let remaining = &data[consumed..];

            if remaining.is_empty() {
                // Can't peek yet — optimistically assume data response.
                self.phase = ResponsePhase::MatchHeader {
                    buf: [0; 2],
                    filled: 0,
                    inner,
                };
                return Err(self);
            }

            let peek = remaining[0];

            if peek < 0x80 {
                // Byte < 0x80 can't be a short-command identifier →
                // status-only frame. Consume only the 1 status byte.
                return Ok((None, consumed));
            }

            if peek == self.expected_id {
                // Our data structure — enter matching header phase.
                self.phase = ResponsePhase::MatchHeader {
                    buf: [0; 2],
                    filled: 0,
                    inner,
                };
            } else {
                // Different command's data structure — enter skip phase.
                self.phase = ResponsePhase::SkipHeader {
                    buf: [0; 2],
                    filled: 0,
                    inner,
                };
            }
        }

        // ── Phase: read header (id + count) for MATCHING response ────────
        if let ResponsePhase::MatchHeader {
            ref mut buf,
            ref mut filled,
            ..
        } = self.phase
        {
            let remaining = &data[consumed..];
            let need = 2 - *filled;
            let take = need.min(remaining.len());
            buf[*filled..*filled + take].copy_from_slice(&remaining[..take]);
            *filled += take;
            consumed += take;
            if *filled < 2 {
                return Err(self);
            }
        }
        // Transition MatchHeader → MatchData
        if matches!(self.phase, ResponsePhase::MatchHeader { filled: 2, .. }) {
            if let ResponsePhase::MatchHeader { inner, .. } = self.phase {
                self.phase = ResponsePhase::MatchData(inner);
            }
        }

        // ── Phase: delegate to inner decoder for matching data ────────────
        if let ResponsePhase::MatchData(inner) = self.phase {
            let remaining = &data[consumed..];
            return match inner.feed(remaining) {
                Ok((output, n)) => Ok((Some(output), consumed + n)),
                Err(inner) => {
                    self.phase = ResponsePhase::MatchData(inner);
                    Err(self)
                }
            };
        }

        // ── Phase: read header (id + count) for NON-matching response ────
        if let ResponsePhase::SkipHeader {
            ref mut buf,
            ref mut filled,
            ..
        } = self.phase
        {
            let remaining = &data[consumed..];
            let need = 2 - *filled;
            let take = need.min(remaining.len());
            buf[*filled..*filled + take].copy_from_slice(&remaining[..take]);
            *filled += take;
            consumed += take;
            if *filled < 2 {
                return Err(self);
            }
        }
        // Transition SkipHeader → SkipData (using count from buf[1])
        if matches!(self.phase, ResponsePhase::SkipHeader { filled: 2, .. }) {
            if let ResponsePhase::SkipHeader { buf, inner, .. } = self.phase {
                self.phase = ResponsePhase::SkipData {
                    remaining: buf[1] as usize,
                    inner,
                };
            }
        }

        // ── Phase: skip N data bytes of non-matching data structure ──────
        if let ResponsePhase::SkipData {
            ref mut remaining, ..
        } = self.phase
        {
            let available = data.len() - consumed;
            let skip = (*remaining).min(available);
            consumed += skip;
            *remaining -= skip;
            if *remaining > 0 {
                return Err(self);
            }
            // Done skipping — return None (no matching data).
            return Ok((None, consumed));
        }

        unreachable!()
    }
}
