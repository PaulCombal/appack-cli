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

use serde::{Deserialize, Serialize};

pub mod app_build_config;
pub mod app_installed;
pub mod local_settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDesktopEntry {
    pub entry: String,
    pub icon: String,
    pub rdp_args: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppSnapshotTriggerMode {
    OnClose,
    Never,
    NeverLoad,
}
