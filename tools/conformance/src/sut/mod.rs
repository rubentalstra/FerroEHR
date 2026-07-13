//! Multi-SUT support: the descriptor (config, not code) and the built-in
//! first-class targets. One case universe drives every SUT; per-SUT wire
//! facts live here, never in suite literals (register 90 §3/§8.4).

pub mod builtin;
pub mod descriptor;
