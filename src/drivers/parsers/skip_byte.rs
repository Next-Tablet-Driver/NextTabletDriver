use crate::drivers::TabletData;
use crate::drivers::parsers::{ReportParser, fallback::FallbackParser};

pub struct SkipByteParser;

impl ReportParser for SkipByteParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        if data.is_empty() {
            return None;
        }

        let fallback = FallbackParser;
        let mut parsed = data.get(1..).and_then(|sub| fallback.parse(sub));
        if let Some(ref mut p) = parsed {
            // Restore raw data to include the skipped byte
            p.set_raw(data);
        }
        parsed
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_byte_tablet() -> Result<(), Box<dyn std::error::Error>> {
        let parser = SkipByteParser;
        // Prefix with 0x05, then standard fallback data
        let data: [u8; 9] = [0x05, 0x02, 0x01, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00];
        let report = parser
            .parse(&data)
            .ok_or("SkipByte parser failed to parse packet")?;
        assert_eq!(report.status, crate::drivers::TabletStatus::Contact);
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        Ok(())
    }

    #[test]
    fn an_empty_report_is_rejected() {
        assert!(SkipByteParser.parse(&[]).is_none());
    }

    #[test]
    fn a_single_byte_leaves_nothing_to_parse() {
        assert!(SkipByteParser.parse(&[0x05]).is_none());
    }

    #[test]
    fn the_raw_data_keeps_the_skipped_byte() {
        let data = [0x05, 0x02, 0x01, 0x02, 0x01, 0x04, 0x03, 0x01, 0x00];
        let parsed = SkipByteParser.parse(&data).unwrap();
        assert_eq!(parsed.raw_len, 9);
        assert_eq!(parsed.raw_data[0], 0x05);
    }
}
