//! Parsed model of `PropertyUnitData.xml`: physical properties and their
//! units, used by `DV_QUANTITY` validation in later phases (P11).
//!
//! Document shape:
//!
//! ```xml
//! <PropertyUnits xmlns="http://tempuri.org/PropertyUnits.xsd">
//!   <Property id="0" Text="Length" openEHR="122" />
//!   <Unit property_id="0" Text="cm" name="centimeter" conversion="1"
//!         coefficient="-2" primary="false" UCUM="cm"/>
//! </PropertyUnits>
//! ```

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use crate::error::TerminologyError;

const SOURCE_NAME: &str = "PropertyUnitData.xml";

/// The full property/unit table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyUnitData {
    /// `<Property>` rows in document order.
    pub properties: Vec<Property>,
    /// `<Unit>` rows in document order.
    pub units: Vec<Unit>,
}

/// One `<Property>` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// `id` attribute — the file-internal key `<Unit property_id>` points at.
    pub id: String,
    /// `Text` attribute (e.g. `Length`).
    pub text: String,
    /// `openEHR` attribute — the concept id of this property in the
    /// `property` group of the openEHR terminology (e.g. `122`).
    pub openehr_code: String,
}

/// One `<Unit>` row. Numeric fields stay strings: this phase transcribes the
/// data faithfully; unit conversion math belongs to the validation phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// `property_id` attribute, joining [`Property::id`].
    pub property_id: String,
    /// `Text` attribute — the unit symbol as displayed (e.g. `cm`).
    pub text: String,
    /// `name` attribute — the long name (e.g. `centimeter`).
    pub name: Option<String>,
    /// `conversion` attribute (factor to the primary unit).
    pub conversion: Option<String>,
    /// `coefficient` attribute (power-of-ten exponent).
    pub coefficient: Option<String>,
    /// `primary` attribute.
    pub primary: bool,
    /// `UCUM` attribute — the UCUM code where one exists.
    pub ucum: Option<String>,
}

fn attribute(
    element: &BytesStart<'_>,
    name: &'static str,
) -> Result<Option<String>, TerminologyError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|source| TerminologyError::Attribute {
            source_name: SOURCE_NAME,
            source,
        })?;
        if attr.key.as_ref() == name.as_bytes() {
            // PropertyUnitData.xml declares <?xml version="1.0"?>.
            let value = attr
                .normalized_value(XmlVersion::Explicit1_0)
                .map_err(|source| TerminologyError::Xml {
                    source_name: SOURCE_NAME,
                    source,
                })?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

fn required(
    element: &BytesStart<'_>,
    name: &'static str,
    element_name: &'static str,
) -> Result<String, TerminologyError> {
    attribute(element, name)?.ok_or(TerminologyError::MissingAttribute {
        source_name: SOURCE_NAME,
        element: element_name,
        attribute: name,
    })
}

/// Parses the bundled `PropertyUnitData.xml`.
///
/// # Errors
///
/// [`TerminologyError`] when the XML is malformed or a required attribute
/// is missing from a `<Property>`/`<Unit>` row.
pub fn parse_property_unit_data(xml: &str) -> Result<PropertyUnitData, TerminologyError> {
    let mut reader = Reader::from_str(xml);
    let mut data = PropertyUnitData {
        properties: Vec::new(),
        units: Vec::new(),
    };

    loop {
        let event = reader
            .read_event()
            .map_err(|source| TerminologyError::Xml {
                source_name: SOURCE_NAME,
                source,
            })?;
        match event {
            Event::Start(e) | Event::Empty(e) => match e.name().as_ref() {
                b"Property" => data.properties.push(Property {
                    id: required(&e, "id", "Property")?,
                    text: required(&e, "Text", "Property")?,
                    openehr_code: required(&e, "openEHR", "Property")?,
                }),
                b"Unit" => data.units.push(Unit {
                    property_id: required(&e, "property_id", "Unit")?,
                    text: required(&e, "Text", "Unit")?,
                    name: attribute(&e, "name")?,
                    conversion: attribute(&e, "conversion")?,
                    coefficient: attribute(&e, "coefficient")?,
                    primary: attribute(&e, "primary")?.as_deref() == Some("true"),
                    ucum: attribute(&e, "UCUM")?,
                }),
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(data)
}

impl PropertyUnitData {
    /// The property row whose openEHR terminology concept id is
    /// `openehr_code` (a code from the `property` group, e.g. `124` = Mass).
    #[must_use]
    pub fn property_for_openehr_code(&self, openehr_code: &str) -> Option<&Property> {
        self.properties
            .iter()
            .find(|p| p.openehr_code == openehr_code)
    }

    /// All units of the property identified by its openEHR concept id.
    #[must_use]
    pub fn units_for_openehr_property(&self, openehr_code: &str) -> Vec<&Unit> {
        match self.property_for_openehr_code(openehr_code) {
            Some(property) => self
                .units
                .iter()
                .filter(|u| u.property_id == property.id)
                .collect(),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets;

    #[test]
    fn parses_the_bundled_property_unit_data() {
        let data = parse_property_unit_data(assets::PROPERTY_UNIT_DATA)
            .expect("bundled PropertyUnitData.xml must parse");
        assert!(!data.properties.is_empty());
        assert_eq!(data.units.len(), 521);
    }

    #[test]
    fn mass_units_include_kilogram() {
        let data = parse_property_unit_data(assets::PROPERTY_UNIT_DATA)
            .expect("bundled PropertyUnitData.xml must parse");
        // openEHR concept 124 = Mass (property group).
        let mass = data
            .property_for_openehr_code("124")
            .expect("Mass property");
        assert_eq!(mass.text, "Mass");
        let units = data.units_for_openehr_property("124");
        assert!(units.iter().any(|u| u.text == "kg"));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: TERM Release-3.0.0 computable/XML/PropertyUnitData.xml (vendored) — specifications-TERM @ d45ef3e
//   source_loc: assets/PropertyUnitData.xml + assets/schema/PropertyUnitData.xsd
//   confidence: high
//   todos: 0
//   note: values kept as strings; conversion math is P11's concern (DV_QUANTITY validation)
// ─────────────────────────────────────────────
