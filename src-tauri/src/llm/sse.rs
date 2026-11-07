//! An incremental parser for server-sent events, the streaming format of every
//! provider (spec §8). Chunks arrive split at arbitrary byte positions; the
//! parser keeps the unfinished line until the rest arrives.

use crate::error::{AppError, AppErrorCode, AppResult};

/// Longest line kept while waiting for its end. A streamed event is a few
/// hundred bytes; a server sending more without a newline is broken.
const MAX_LINE: usize = 1024 * 1024;

/// One dispatched event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// The `event:` field; empty for the default `message` type.
    pub event: String,
    /// The `data:` lines, joined with `\n`.
    pub data: String,
}

/// Collects lines into events.
#[derive(Debug, Default)]
pub struct SseParser {
    line: Vec<u8>,
    event: String,
    data: Option<String>,
}

impl SseParser {
    /// Adds a chunk of the body and returns the events it completes.
    pub fn push(&mut self, chunk: &[u8]) -> AppResult<Vec<SseEvent>> {
        let mut events = Vec::new();
        for &byte in chunk {
            if byte == b'\n' {
                let mut line = std::mem::take(&mut self.line);
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                self.line_done(&line, &mut events);
            } else {
                if self.line.len() >= MAX_LINE {
                    return Err(AppError::new(
                        AppErrorCode::ProviderFailed,
                        "a streamed line is too long",
                    ));
                }
                self.line.push(byte);
            }
        }
        Ok(events)
    }

    /// Ends the body: an event the server did not close with an empty line is
    /// still dispatched.
    pub fn finish(&mut self) -> Option<SseEvent> {
        let mut events = Vec::new();
        if !self.line.is_empty() {
            let line = std::mem::take(&mut self.line);
            self.line_done(&line, &mut events);
        }
        self.line_done(b"", &mut events);
        events.pop()
    }

    fn line_done(&mut self, line: &[u8], events: &mut Vec<SseEvent>) {
        if line.is_empty() {
            if let Some(data) = self.data.take() {
                events.push(SseEvent {
                    event: std::mem::take(&mut self.event),
                    data,
                });
            }
            self.event.clear();
            return;
        }
        if line.first() == Some(&b':') {
            return;
        }
        let line = String::from_utf8_lossy(line);
        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (line.as_ref(), ""),
        };
        match field {
            "event" => value.clone_into(&mut self.event),
            "data" => match &mut self.data {
                Some(data) => {
                    data.push('\n');
                    data.push_str(value);
                }
                None => self.data = Some(value.to_owned()),
            },
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(event: &str, data: &str) -> SseEvent {
        SseEvent {
            event: event.into(),
            data: data.into(),
        }
    }

    #[test]
    fn parses_events_split_across_chunks() {
        let mut parser = SseParser::default();
        let body = "event: message_start\ndata: {\"a\":1}\n\n: ping\r\ndata: {\"b\"\r\ndata: :2}\r\n\r\ndata: [DONE]\n\n";
        let mut events = Vec::new();
        for chunk in body.as_bytes().chunks(5) {
            events.extend(parser.push(chunk).unwrap());
        }
        assert_eq!(
            events,
            vec![
                event("message_start", "{\"a\":1}"),
                event("", "{\"b\"\n:2}"),
                event("", "[DONE]"),
            ]
        );
        assert_eq!(parser.finish(), None);
    }

    #[test]
    fn keeps_utf8_split_inside_a_character() {
        let mut parser = SseParser::default();
        let body = "data: صبر\n\n".as_bytes();
        let (a, b) = body.split_at(8);
        assert!(parser.push(a).unwrap().is_empty());
        assert_eq!(parser.push(b).unwrap(), vec![event("", "صبر")]);
    }

    #[test]
    fn dispatches_an_unterminated_event_at_the_end() {
        let mut parser = SseParser::default();
        assert!(parser.push(b"data: last").unwrap().is_empty());
        assert_eq!(parser.finish(), Some(event("", "last")));
    }

    #[test]
    fn refuses_endless_lines() {
        let mut parser = SseParser::default();
        let err = parser.push(&vec![b'x'; MAX_LINE + 1]).unwrap_err();
        assert_eq!(err.code, AppErrorCode::ProviderFailed);
    }
}
