//! The Better `web-template` model and builder.
//!
//! [`build_web_template`] turns a parsed [`openehr_its::opt14::OperationalTemplate`]
//! into a [`WebTemplate`] (format version `"2.3"`). See [`builder`] for the walk
//! and the recorded scope boundaries.

mod builder;
mod id;
mod inputs;
mod model;

pub use builder::build_web_template;
pub(crate) use inputs::PROPORTION_KINDS;
pub use model::{
    WebTemplate, WebTemplateArchetypeSlot, WebTemplateBindingCodedValue, WebTemplateCardinality,
    WebTemplateClosedAttribute, WebTemplateCodeList, WebTemplateCodedValue, WebTemplateExistence,
    WebTemplateInput, WebTemplateInputType, WebTemplateNode, WebTemplateRange, WebTemplateSlot,
    WebTemplateStructuralStub, WebTemplateValidation,
};
