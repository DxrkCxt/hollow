// SPDX-License-Identifier: GPL-3.0-only

//! A closure that writes
//! version-dependent payload into a buffer.

use crate::ByteMessage;
use crate::registry::Version;

pub type MetadataWriter = Box<dyn Fn(&mut ByteMessage, Version) + Send + Sync>;
