//! Convert a canonical-JSON RM instance to its canonical-XML form — the
//! offline generator for XML corpus fixtures (e.g. the CNF catalogue's
//! request-body fixtures), so every committed XML fixture is produced by the
//! same gate-proven codec pair that serves the wire.
//!
//! Usage: `cargo run -p openehr-its --example canonical_convert -- <in.json> <out.xml>`
//!
//! The concrete RM type is dispatched on the document's `_type`
//! (ITS-JSON: the `_type` self-tag names the RM class); the XML root element
//! is the lower-snake class name and the namespace is the canonical
//! `http://schemas.openehr.org/v1` (ITS-XML; vendored XSDs at
//! `crates/openehr-its/schemas/xml/`).

use std::error::Error;

use openehr_rm::common::directory::folder::Folder;
use openehr_rm::composition::composition::Composition;
use openehr_rm::demographic::agent::Agent;
use openehr_rm::demographic::group::Group;
use openehr_rm::demographic::organisation::Organisation;
use openehr_rm::demographic::person::Person;
use openehr_rm::demographic::role::Role;
use openehr_rm::ehr::ehr_status::EhrStatus;

use openehr_its::json_codec::runtime::FromJson;
use openehr_its::xml::ToXml;

fn convert<T: FromJson + ToXml>(
    value: &serde_json::Value,
    root_tag: &str,
) -> Result<String, Box<dyn Error>> {
    let typed: T = openehr_its::json::from_canonical_value(value)?;
    Ok(openehr_its::xml::to_canonical_xml(&typed, root_tag)?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(output)) = (args.next(), args.next()) else {
        return Err("usage: canonical_convert <in.json> <out.xml>".into());
    };
    let text = std::fs::read_to_string(&input)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let rm_type = value
        .get("_type")
        .and_then(serde_json::Value::as_str)
        .ok_or("input document carries no _type self-tag")?;
    let xml = match rm_type {
        "COMPOSITION" => convert::<Composition>(&value, "composition")?,
        "EHR_STATUS" => convert::<EhrStatus>(&value, "ehr_status")?,
        "FOLDER" => convert::<Folder>(&value, "folder")?,
        "PERSON" => convert::<Person>(&value, "person")?,
        "AGENT" => convert::<Agent>(&value, "agent")?,
        "GROUP" => convert::<Group>(&value, "group")?,
        "ORGANISATION" => convert::<Organisation>(&value, "organisation")?,
        "ROLE" => convert::<Role>(&value, "role")?,
        other => return Err(format!("unsupported root _type {other}").into()),
    };
    std::fs::write(&output, xml)?;
    Ok(())
}
