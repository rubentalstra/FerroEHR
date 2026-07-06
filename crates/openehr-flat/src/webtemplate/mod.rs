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
pub use model::{
    WebTemplate, WebTemplateBindingCodedValue, WebTemplateCardinality, WebTemplateCodedValue,
    WebTemplateExistence, WebTemplateInput, WebTemplateInputType, WebTemplateNode,
    WebTemplateRange, WebTemplateValidation,
};
