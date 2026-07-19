//! Stage 4 — RENDER. The text producers: given the analysed model and the
//! planned shapes, emit deterministic, byte-stable Rust source. This is the
//! only stage that writes code text. [`emit`] renders the BMM spec crates;
//! [`emit_json`] the canonical-JSON `ToJson` codecs; [`emit_xml`] the
//! canonical-XML codecs; [`emit_rest`] the ITS-REST contract;
//! [`emit_rm_model`] the static RM attribute/type model; [`emit_opt`] the OPT
//! 1.4 model; [`naming`] is the shared identifier-casing helper.

pub(crate) mod emit;
pub(crate) mod emit_json;
pub(crate) mod emit_opt;
pub(crate) mod emit_rest;
pub(crate) mod emit_rm_model;
pub(crate) mod emit_xml;
pub(crate) mod naming;
