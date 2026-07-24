//! The Web Template model and builder.
//!
//! A Web Template is the processed representation of an operational template —
//! simplified node identifiers, AQL paths, input type definitions, localized
//! labels, and multiplicity constraints — defined by `ITS-REST
//! simplified_formats master04-basic_concepts.adoc` §"Web Template Metadata".
//! [`build_web_template`] turns a parsed
//! [`crate::opt14::OperationalTemplate`] into a [`WebTemplate`]. See
//! [`builder`] for the walk and the recorded scope boundaries, [`id`] for the
//! master04 §"Node ID Generation Rules" algorithm, and [`inputs`] for the
//! per-RM-type input mapping.

mod builder;
mod builder_am24;
mod id;
mod inputs;
mod model;
mod shape;

pub use builder::build_web_template;
pub use builder_am24::build_web_template_am24;
pub(crate) use inputs::PROPORTION_KINDS;
pub use model::{
    CodedName, WebTemplate, WebTemplateArchetypeSlot, WebTemplateBindingCodedValue,
    WebTemplateCardinality, WebTemplateClosedAttribute, WebTemplateCodeList, WebTemplateCodedValue,
    WebTemplateExistence, WebTemplateInput, WebTemplateInputType, WebTemplateNode,
    WebTemplateRange, WebTemplateSlot, WebTemplateStructuralStub, WebTemplateValidation,
};
