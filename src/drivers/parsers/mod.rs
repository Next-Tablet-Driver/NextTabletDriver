pub mod acepen;
pub mod bosto;
pub mod fallback;
pub mod floogoo;
pub mod genius;
pub mod huion;
pub mod lifetec;
pub mod robotpen;
pub mod skip_byte;
pub mod tenmoon;
pub mod uclogic;
pub mod veikk;
pub mod viewsonic;
pub mod wacom;
pub mod xencelabs;
pub mod xenx;
pub mod xp_pen;

use crate::drivers::TabletData;

pub trait ReportParser: Send + Sync {
    fn parse(&self, data: &[u8]) -> Option<TabletData>;
}

#[must_use]
pub fn create_parser(parser_name: &str) -> Box<dyn ReportParser> {
    match parser_name {
        name if name.contains("Acepen") => Box::new(acepen::AcepenParser::new()),
        name if name.contains("Bosto") => Box::new(bosto::BostoParser),
        name if name.contains("FlooGoo") => Box::new(floogoo::FlooGooParser),
        name if name.contains("GeniusReportParserV2") => Box::new(genius::GeniusParserV2),
        name if name.contains("Genius") => Box::new(genius::GeniusParserV1),
        name if name.contains("Lifetec") => Box::new(lifetec::LifetecParser),
        name if name.contains("RobotPen") => Box::new(robotpen::RobotPenParser),
        name if name.contains("SkipByteTabletReportParser") => Box::new(skip_byte::SkipByteParser),
        name if name.contains("10moon") => Box::new(tenmoon::TenMoonParser),
        name if name.contains("UCLogicV1") => Box::new(uclogic::UCLogicV1Parser),
        name if name.contains("UCLogicV2") => Box::new(uclogic::UCLogicV2Parser),
        name if name.contains("UCLogicTilt") => Box::new(uclogic::UCLogicTiltParser),
        name if name.contains("UCLogic") => Box::new(uclogic::UCLogicParser),
        name if name.contains("WoodPad") => Box::new(viewsonic::ViewSonicParser),
        name if name.contains("XenceLabs") => Box::new(xencelabs::XenceLabsParser),
        name if name.contains("XENX") => Box::new(xenx::XenxParser),
        name if name.contains("XP_PenGen2") => Box::new(xp_pen::extended::XpPenGen2Parser),
        name if name.contains("XP_PenDeco03") => Box::new(xp_pen::extended::XpPenDeco03Parser),
        name if name.contains("XP_PenOffsetPressure") => {
            Box::new(xp_pen::extended::XpPenOffsetPressureParser)
        }
        name if name.contains("XP_PenOffsetAux") => {
            Box::new(xp_pen::extended::XpPenOffsetAuxParser)
        }
        name if name.contains("XP_PenReportParser") => {
            Box::new(xp_pen::star_g640::XpPenStarG640Parser)
        }
        name if name.contains("XP_Pen") => Box::new(xp_pen::extended::XpPenParser),
        name if name.contains("VeikkA15") => Box::new(veikk::generic::VeikkA15Parser),
        name if name.contains("VeikkTilt") => Box::new(veikk::generic::VeikkTiltParser),
        name if name.contains("VeikkV1") => Box::new(veikk::generic::VeikkV1Parser),
        name if name.contains("Veikk") => Box::new(veikk::generic::VeikkParser),
        name if name.contains("Inspiroy") || name.contains("Giano") => {
            Box::new(huion::inspiroy::InspiroyParser)
        }
        // Wacom Parsers
        name if name.contains("Wacom.Bamboo.BambooReportParser") => {
            Box::new(wacom::bamboo::BambooParser)
        }
        name if name.contains("Wacom.BambooPad.BambooPadReportParser") => {
            Box::new(wacom::bamboo_pad::BambooPadParser)
        }
        name if name.contains("Wacom.BambooV2.BambooV2AuxReportParser") => {
            Box::new(wacom::bamboo_v2::BambooV2AuxParser)
        }
        name if name.contains("Wacom.CintiqV1.CintiqV1ReportParser") => {
            Box::new(wacom::cintiq_v1::CintiqV1Parser::new())
        }
        name if name.contains("Wacom.Graphire.GraphireReportParser") => {
            Box::new(wacom::graphire::GraphireParser)
        }
        name if name.contains("Wacom.Intuos.WacomDriverIntuosReportParser") => {
            Box::new(wacom::intuos::WacomDriverIntuosParser)
        }
        name if name.contains("Wacom.Intuos.IntuosReportParser") => {
            Box::new(wacom::intuos::IntuosParser)
        }
        name if name.contains("Wacom.IntuosV1.IntuosV1ReportParser") => {
            Box::new(wacom::intuos_v1::IntuosV1Parser::new())
        }
        name if name.contains("Wacom.IntuosV1.WacomDriverIntuosV1ReportParser") => {
            Box::new(wacom::intuos_v1::WacomDriverIntuosV1Parser::new())
        }
        name if name.contains("Wacom.IntuosV2.IntuosV2ReportParser")
            || name.contains("Wacom.IntuosV2.WacomDriverIntuosV2ReportParser") =>
        {
            Box::new(wacom::intuos_v2::WacomDriverIntuosV2Parser::new())
        }
        name if name.contains("Wacom.IntuosV3.IntuosV3ReportParser")
            || name.contains("Wacom.IntuosV3.WacomDriverIntuosV3ReportParser") =>
        {
            Box::new(wacom::intuos_v3::WacomDriverIntuosV3Parser::new())
        }
        name if name.contains("Wacom.Intuos4.Intuos4ReportParser")
            || name.contains("Wacom.Intuos4.WacomDriverIntuos4ReportParser") =>
        {
            Box::new(wacom::intuos4::WacomDriverIntuos4Parser::new())
        }
        name if name.contains("Wacom.IntuosPro.IntuosProReportParser")
            || name.contains("Wacom.IntuosPro.WacomDriverIntuosProReportParser") =>
        {
            Box::new(wacom::intuos_pro::WacomDriverIntuosProParser::new())
        }
        name if name.contains("Wacom.Intuos3.Intuos3ReportParser")
            || name.contains("Wacom.Intuos3.WacomDriverIntuos3ReportParser") =>
        {
            Box::new(wacom::intuos3::WacomDriverIntuos3Parser::new())
        }
        name if name.contains("Wacom.Intuos3.Intuos3ExtraAuxReportParser") => {
            Box::new(wacom::intuos3::Intuos3ExtraAuxParser::new())
        }
        name if name.contains("Wacom.PL.PLReportParser") => Box::new(wacom::pl::PLParser::new()),
        name if name.contains("Wacom.PTU.PTUReportParser") => Box::new(wacom::ptu::PTUParser),
        name if name.contains("Wacom.Wacom64bAuxReportParser") => {
            Box::new(wacom::misc::Wacom64bAuxParser)
        }

        name if name.contains("Gaomon") || name.contains("Tablet") => {
            Box::new(fallback::FallbackParser)
        }
        _ => {
            if !parser_name.is_empty() {
                log::warn!(
                    target: "Driver",
                    "Unknown report parser '{}' specified in configuration. Falling back to FallbackParser.",
                    parser_name
                );
            }
            Box::new(fallback::FallbackParser)
        }
    }
}
