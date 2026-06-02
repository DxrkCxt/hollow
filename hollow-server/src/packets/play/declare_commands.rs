// SPDX-License-Identifier: GPL-3.0-only

//! Declare Commands packet (1.13+).

use hollow_protocol::ByteMessage;
use hollow_protocol::packet::PacketOut;
use hollow_protocol::registry::{PacketKind, Version};

#[derive(Clone, Debug, Default)]
pub struct PacketDeclareCommands {
    pub commands: Vec<String>,
}

impl PacketOut for PacketDeclareCommands {
    fn encode(&self, buf: &mut ByteMessage, _version: Version) {
        let n = self.commands.len();
        // Total nodes: 1 root + n literal nodes + n argument nodes = n*2 + 1
        buf.write_var_int((n * 2 + 1) as i32);

        // Root node: flags=0 (ROOT), children = first n*2 odd indices (every other starting at 1)
        buf.write_u8(0); // flags: ROOT
        buf.write_var_int(n as i32); // child count
        // The Java loop: for i in 1..=n*2 { write(i); i++ }
        // This writes 1, 3, 5, ... (every other value, skipping the +2 increment)
        let mut i: i32 = 1;
        while i <= (n * 2) as i32 {
            buf.write_var_int(i);
            i += 2;
        }

        // Per-command: literal node + argument node
        let mut idx: i32 = 1;
        for cmd in &self.commands {
            // Literal node: flags = LITERAL(1) | HAS_REDIRECT(0x04) = 0x05
            buf.write_u8(1 | 0x04);
            buf.write_var_int(1);           // 1 child
            buf.write_var_int(idx + 1);     // child index
            buf.write_string(cmd);
            idx += 1;

            // Argument node: flags = ARGUMENT(2) | HAS_REDIRECT(0x04) | SUGGESTS_TYPE(0x10) = 0x16
            buf.write_u8(2 | 0x04 | 0x10);
            buf.write_var_int(1);           // 1 child
            buf.write_var_int(idx);         // child index (points back, creating a cycle)
            buf.write_string("arg");
            buf.write_string("brigadier:string");
            buf.write_var_int(0);           // string type: SINGLE_WORD
            buf.write_string("minecraft:ask_server");
            idx += 1;
        }

        // Root node index
        buf.write_var_int(0);
    }

    fn kind(&self) -> PacketKind {
        PacketKind::DeclareCommands
    }
}
