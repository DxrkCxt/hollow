// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketUpdateTags {
    // Map<registryKey, Map<tagName, List<id>>>
    // TODO: tag ordering — HashMap iteration order is nondeterministic (mirrors
    // Java HashMap, whose order is also nondeterministic). A golden-test step
    // will need to reconcile ordering if byte-exact parity is required.
    pub tags: HashMap<String, HashMap<String, Vec<i32>>>,
}

impl PacketOut for PacketUpdateTags {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        buf.write_var_int(self.tags.len() as i32);
        for (registry_key, sub_tags) in &self.tags {
            buf.write_string(registry_key);

            buf.write_var_int(sub_tags.len() as i32);
            for (tag_name, ids) in sub_tags {
                buf.write_string(tag_name);

                buf.write_var_int(ids.len() as i32);
                for &id in ids {
                    buf.write_var_int(id);
                }
            }
        }
    }

    fn kind(&self) -> PacketKind {
        PacketKind::UpdateTags
    }
}
