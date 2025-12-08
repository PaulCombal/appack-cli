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
use anyhow::{Result, anyhow};
use std::fs;

pub fn uninstall_appack(settings: &AppPackLocalSettings, app_id: &str) -> Result<()> {
    let mut installed = settings.get_installed()?;

    let entry: Vec<_> = installed
        .installed
        .iter()
        .filter(|e| e.id == app_id)
        .collect();

    if entry.len() == 0 {
        println!("AppPack not installed: {}", app_id);
        Err(anyhow!("AppPack not installed"))?
    }

    if entry.len() > 1 {
        println!("Multiple versions installed: {}", app_id);
        Err(anyhow!("Multiple versions installed"))?
    }

    let entry = entry[0];
    let entry_version = entry.version.clone();
    let entry_id = entry.id.clone();

    // 1. Remove desktop entries
    if let Some(entries) = &entry.desktop_entries {
        for desktop_entry in entries {
            let entry_path = settings
                .desktop_entries_dir
                .join(format!("{entry_version}_{}", desktop_entry.entry));
            if !entry_path.exists() {
                println!("Desktop entry not found: {}", entry_path.display());
                continue;
            }
            fs::remove_file(&entry_path)?;

            // We do not need to delete desktop icons as they are in the app dir
        }
    }

    // 3. Delete AppPack directory
    {
        let appack_dir = settings.home_dir.join(entry_id).join(entry_version);
        if !appack_dir.exists() {
            println!("AppPack dir does not exist: {appack_dir:?}");
            return Err(anyhow!("AppPack dir does not exist"))?;
        }

        fs::remove_dir_all(&appack_dir)?;
    }

    installed.installed.retain(|e| e.id != app_id);
    settings.save_installed(installed)?;

    Ok(())
}
