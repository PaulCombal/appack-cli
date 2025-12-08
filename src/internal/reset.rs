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

use crate::types::local_settings::AppPackLocalSettings;
use anyhow::Result;
use anyhow::{Context, anyhow};
use std::process::Command;

pub fn reset(settings: &AppPackLocalSettings, id: String, version: Option<&str>) -> Result<()> {
    let app_installed = settings
        .get_app_installed(&id, version)
        .context("Failed to get installed AppPack")?;
    let app_installed_home = settings.get_app_home_dir(&app_installed);
    let image_name = app_installed.image.clone();
    let image_path = app_installed_home.join(image_name);

    let result = Command::new("qemu-img")
        .arg("snapshot")
        .arg("-d")
        .arg("appack-onclose")
        .arg(&image_path)
        .status()
        .context("Failed to delete snapshot 'appack-onclose'")?;

    if !result.success() {
        return Err(anyhow!(
            "Failed to reset the AppPack. Make sure the AppPack is NOT running."
        ))
        .context("Failed to delete snapshot 'appack-onclose'");
    }

    Ok(())
}
