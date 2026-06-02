// SPDX-License-Identifier: GPL-3.0-only

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::large_enum_variant)]

pub mod dimension;
pub mod dimension_registry;
pub mod dimension_type;
pub mod versioned_dimension;

pub use dimension::Dimension;
pub use dimension_registry::DimensionRegistry;
pub use dimension_type::DimensionType;
pub use versioned_dimension::VersionedDimension;
