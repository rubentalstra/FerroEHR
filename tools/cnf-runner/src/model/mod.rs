// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The typed artifact model — one module per artifact family.

pub mod assertion;
pub mod binding;
pub mod capability;
pub mod case;
pub mod corpus;
pub(crate) mod de;
pub mod register;
pub mod value;
pub mod vocab_files;
pub mod wire_surface;
