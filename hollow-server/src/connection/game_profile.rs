// SPDX-License-Identifier: GPL-3.0-only

use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct GameProfile {
    pub uuid: Option<Uuid>,
    pub username: Option<String>,
}
