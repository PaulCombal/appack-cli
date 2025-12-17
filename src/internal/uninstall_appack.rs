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

pub fn uninstall_appack(
    settings: &AppPackLocalSettings,
    app_id: &str,
    version: Option<&str>,
) -> Result<()> {
    let mut installed = settings.get_installed()?;

    let app_entries: Vec<_> = if let Some(version) = version {
        installed
            .installed
            .iter()
            .filter(|e| e.id == app_id && e.version == version)
            .collect()
    } else {
        installed
            .installed
            .iter()
            .filter(|e| e.id == app_id)
            .collect()
    };

    if app_entries.len() == 0 {
        println!("AppPack not installed: {}", app_id);
        Err(anyhow!("AppPack not installed"))?
    }

    if app_entries.len() > 1 {
        println!("Multiple versions installed: {}", app_id);
        Err(anyhow!("Multiple versions installed"))?
    }

    let app_entry = app_entries[0];
    let entry_version = app_entry.version.clone();
    let entry_id = app_entry.id.clone();

    // 1. Remove desktop entries
    if let Some(entries) = &app_entry.desktop_entries {
        for desktop_entry in entries {
            let entry_path = settings.get_desktop_entry_path(app_entry, desktop_entry);
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

pub fn uninstall_all_appacks(settings: &AppPackLocalSettings) -> Result<()> {
    let installed = settings.get_installed()?;
    for entry in installed.installed {
        uninstall_appack(settings, &entry.id, Some(&entry.version))?;
    }
    Ok(())
}
