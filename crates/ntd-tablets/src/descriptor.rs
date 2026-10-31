//! The static form of a tablet definition: what `ntd-codegen` writes into `generated/`.
//!
//! A descriptor borrows everything from the binary, so the embedded catalogue needs no parsing and
//! no allocation until a definition is turned into a [`TabletConfiguration`].

use crate::config::{
    Attributes, AuxiliarySpecs, DigitizerIdentifier, DigitizerSpecs, PenSpecs, Specifications,
    TabletConfiguration,
};
use crate::generated::{BY_ID, TABLETS};
use std::collections::HashMap;

/// A (`VendorID`, `ProductID`) key and the places where it occurs: (index of the tablet, index of
/// its digitizer).
pub(crate) type IdEntry = ((u16, u16), &'static [(u16, u8)]);

/// One tablet definition.
#[derive(Debug, Clone, Copy)]
pub struct TabletDescriptor {
    pub(crate) name: &'static str,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) max_x: f32,
    pub(crate) max_y: f32,
    pub(crate) max_pressure: u16,
    pub(crate) pen_button_count: Option<u8>,
    pub(crate) auxiliary_button_count: Option<u8>,
    /// The definition has an `Attributes` object, possibly empty.
    pub(crate) has_attributes: bool,
    pub(crate) interface: Option<&'static str>,
    pub(crate) libinputoverride: Option<&'static str>,
    pub(crate) digitizers: &'static [DigitizerDescriptor],
}

/// One way to recognise a tablet on the bus, and how to talk to it.
#[derive(Debug, Clone, Copy)]
pub struct DigitizerDescriptor {
    pub(crate) vendor_id: u16,
    pub(crate) product_id: u16,
    pub(crate) input_report_length: Option<usize>,
    pub(crate) output_report_length: Option<usize>,
    /// String descriptor index and the pattern it must match.
    pub(crate) device_strings: &'static [(u8, &'static str)],
    pub(crate) report_parser: &'static str,
    pub(crate) output_init_report: Option<&'static [&'static str]>,
    pub(crate) feature_init_report: Option<&'static [&'static str]>,
    /// The digitizer has an `Attributes` object, possibly empty.
    pub(crate) has_attributes: bool,
    pub(crate) interface: Option<&'static str>,
    pub(crate) libinputoverride: Option<&'static str>,
}

impl TabletDescriptor {
    /// The human-readable name of the tablet.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The definition as the owned configuration the rest of the driver uses.
    #[must_use]
    pub fn to_configuration(&self) -> TabletConfiguration {
        TabletConfiguration {
            name: self.name.to_string(),
            specifications: Specifications {
                digitizer: DigitizerSpecs {
                    width: self.width,
                    height: self.height,
                    max_x: self.max_x,
                    max_y: self.max_y,
                },
                pen: PenSpecs {
                    max_pressure: self.max_pressure,
                    button_count: self.pen_button_count,
                },
                auxiliary_buttons: self.auxiliary_button_count.map(|count| AuxiliarySpecs {
                    button_count: Some(count),
                }),
            },
            digitizer_identifiers: self
                .digitizers
                .iter()
                .map(DigitizerDescriptor::to_identifier)
                .collect(),
            attributes: attributes(self.has_attributes, self.interface, self.libinputoverride),
        }
    }
}

impl DigitizerDescriptor {
    /// The `VendorID` and `ProductID` of the digitizer.
    #[must_use]
    pub const fn vid_pid(&self) -> (u16, u16) {
        (self.vendor_id, self.product_id)
    }

    /// The digitizer as the owned identifier the rest of the driver uses.
    #[must_use]
    pub fn to_identifier(&self) -> DigitizerIdentifier {
        let owned = |reports: Option<&[&str]>| {
            reports.map(|list| list.iter().map(|r| (*r).to_string()).collect())
        };
        DigitizerIdentifier {
            vendor_id: self.vendor_id,
            product_id: self.product_id,
            input_report_length: self.input_report_length,
            output_report_length: self.output_report_length,
            device_strings: (!self.device_strings.is_empty()).then(|| {
                self.device_strings
                    .iter()
                    .map(|(index, pattern)| (*index, (*pattern).to_string()))
                    .collect::<HashMap<_, _>>()
            }),
            report_parser: self.report_parser.to_string(),
            output_init_report: owned(self.output_init_report),
            feature_init_report: owned(self.feature_init_report),
            attributes: attributes(self.has_attributes, self.interface, self.libinputoverride),
        }
    }
}

fn attributes(
    present: bool,
    interface: Option<&str>,
    libinputoverride: Option<&str>,
) -> Option<Attributes> {
    present.then(|| Attributes {
        libinputoverride: libinputoverride.map(str::to_string),
        interface: interface.map(|i| serde_json::Value::String(i.to_string())),
    })
}

/// Every embedded definition, in the order of the catalogue.
#[must_use]
pub const fn all() -> &'static [TabletDescriptor] {
    TABLETS
}

/// The embedded definitions that have a digitizer with this `VendorID` and `ProductID`, each with
/// that digitizer, in the order of the catalogue. A table lookup: nothing is parsed or allocated.
pub fn find(
    vendor_id: u16,
    product_id: u16,
) -> impl Iterator<Item = (&'static TabletDescriptor, &'static DigitizerDescriptor)> {
    let entries = BY_ID
        .binary_search_by_key(&(vendor_id, product_id), |(key, _)| *key)
        .ok()
        .and_then(|at| BY_ID.get(at))
        .map_or(&[][..], |(_, entries)| *entries);
    entries.iter().filter_map(|&(tablet, digitizer)| {
        let tablet = TABLETS.get(usize::from(tablet))?;
        Some((tablet, tablet.digitizers.get(usize::from(digitizer))?))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_is_not_empty_and_names_are_unique() {
        assert!(all().len() > 100);
        let names: std::collections::HashSet<_> =
            all().iter().map(TabletDescriptor::name).collect();
        assert_eq!(names.len(), all().len());
    }

    #[test]
    fn find_returns_every_digitizer_with_the_ids_and_nothing_else() {
        let mut seen = 0;
        for tablet in all() {
            for digitizer in tablet.digitizers {
                let (vid, pid) = digitizer.vid_pid();
                assert!(
                    find(vid, pid)
                        .any(|(t, d)| std::ptr::eq(t, tablet) && std::ptr::eq(d, digitizer)),
                    "{} {vid:04x}:{pid:04x} is not found",
                    tablet.name
                );
                seen += 1;
            }
        }
        let indexed: usize = BY_ID.iter().map(|(_, entries)| entries.len()).sum();
        assert_eq!(indexed, seen);
        assert!(
            BY_ID.windows(2).all(|w| w[0].0 < w[1].0),
            "BY_ID is not sorted"
        );
        assert_eq!(find(0xFFFF, 0xFFFF).count(), 0);
    }

    #[test]
    fn a_descriptor_converts_to_the_configuration_it_describes() {
        let ctl472 = all().iter().find(|t| t.name() == "Wacom CTL-472");
        let config = ctl472.map(TabletDescriptor::to_configuration);
        let Some(config) = config else {
            panic!("Wacom CTL-472 is missing from the catalogue");
        };
        assert_eq!(config.specifications.pen.max_pressure, 2047);
        let digitizer = config.digitizer_identifiers.first();
        assert_eq!(
            digitizer.map(|d| (d.vendor_id, d.product_id)),
            Some((1386, 890))
        );
        assert!(digitizer.is_some_and(|d| d.feature_init_report.is_some()));
    }
}
