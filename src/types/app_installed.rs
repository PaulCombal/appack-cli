// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::types::app_build_config::AppBuildConfig;
use crate::types::{AppDesktopEntry, AppSnapshotTriggerMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstalledAppPackEntry {
    pub id: String,
    pub version: String,
    pub name: String,
    pub image: String,
    pub description: Option<String>,
    pub desktop_entries: Option<Vec<AppDesktopEntry>>,
    pub snapshot_mode: AppSnapshotTriggerMode,
    pub qemu_command: String,
    pub freerdp_command: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InstalledAppPacks {
    #[serde(default)]
    pub installed: Vec<InstalledAppPackEntry>,
}

impl From<AppBuildConfig> for InstalledAppPackEntry {
    fn from(value: AppBuildConfig) -> Self {
        Self {
            id: value.id,
            version: value.version,
            image: value.image,
            name: value.name,
            description: value.description,
            desktop_entries: None,
            qemu_command: format!("{} {}", value.base_command, value.configure_append),
            freerdp_command: value.configure_freerdp,
            snapshot_mode: value.snapshot,
        }
    }
}
