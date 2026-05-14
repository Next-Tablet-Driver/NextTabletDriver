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
            p.raw_data = data
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
        }
        parsed
    }
}

#[cfg(test)]
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
        assert_eq!(report.status, "Contact");
        assert_eq!(report.x, 258);
        assert_eq!(report.pressure, 1);
        Ok(())
    }
}
