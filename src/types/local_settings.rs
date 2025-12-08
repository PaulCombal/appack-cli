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

use crate::types::app_installed::{InstalledAppPackEntry, InstalledAppPacks};
use anyhow::{Context, anyhow};
use std::path::PathBuf;

#[derive(Debug)]
pub struct AppPackLocalSettings {
    pub installed_file: PathBuf,
    pub home_dir: PathBuf,
    pub desktop_entries_dir: PathBuf,
}

impl Default for AppPackLocalSettings {
    #[cfg(not(debug_assertions))]
    fn default() -> Self {
        let snap_home = std::env::var("SNAP_USER_COMMON").unwrap();
        let snap_home = PathBuf::from(snap_home);
        let user_real_home = std::env::var("SNAP_REAL_HOME").unwrap();
        let user_real_home = PathBuf::from(user_real_home);
        Self {
            home_dir: snap_home.clone(),
            installed_file: snap_home.join("installed.yaml"),
            desktop_entries_dir: user_real_home
                .join(".local")
                .join("share")
                .join("applications"),
        }
    }

    #[cfg(debug_assertions)]
    fn default() -> Self {
        let home_str = std::env::var("HOME").unwrap();
        let snap_home = PathBuf::from(&home_str)
            .join("snap")
            .join("appack")
            .join("common");
        let user_real_home = PathBuf::from(home_str);
        Self {
            home_dir: snap_home.clone(),
            installed_file: snap_home.join("installed.yaml"),
            desktop_entries_dir: user_real_home
                .join(".local")
                .join("share")
                .join("applications"),
        }
    }
}

impl AppPackLocalSettings {
    pub fn check_ok(&self) -> anyhow::Result<()> {
        if !self.home_dir.exists() {
            return Err(anyhow!(
                "Home directory does not exist: {}",
                self.home_dir.display()
            ));
        }

        if !self.desktop_entries_dir.exists() {
            return Err(anyhow!(
                "Desktop entries directory does not exist: {}",
                self.desktop_entries_dir.display()
            ).context("Make sure this directory exists and that you installed AppPack using the command line from the README (that the necessary plugs are connected)"));
        }

        Ok(())
    }

    pub fn get_installed(&self) -> anyhow::Result<InstalledAppPacks> {
        let installed_filepath = self.installed_file.clone();

        let installed_app_packs: InstalledAppPacks = if installed_filepath.exists() {
            let content = std::fs::read_to_string(&installed_filepath).context(format!(
                "Failed to read installed file {}",
                installed_filepath.display()
            ))?;
            serde_yaml::from_str(&content).context(format!(
                "Failed to parse installed file {}",
                installed_filepath.display()
            ))?
        } else {
            InstalledAppPacks {
                installed: Vec::new(),
            }
        };

        Ok(installed_app_packs)
    }

    pub fn save_installed(&self, installed_app_packs: InstalledAppPacks) -> anyhow::Result<()> {
        let installed_filepath = self.installed_file.clone();
        let content = serde_yaml::to_string(&installed_app_packs)
            .context("Failed to serialize installed app packs")?;
        std::fs::write(&installed_filepath, content).context(format!(
            "Failed to write installed file {}",
            installed_filepath.display()
        ))?;

        Ok(())
    }

    pub fn get_app_home_dir(&self, app: &InstalledAppPackEntry) -> PathBuf {
        self.home_dir.join(app.id.clone()).join(app.version.clone())
    }

    pub fn get_app_installed(
        &self,
        id: &str,
        version: Option<&str>,
    ) -> anyhow::Result<InstalledAppPackEntry> {
        let all_installed = self
            .get_installed()
            .context("Failed to get installed app packs")?;

        let matches = all_installed.installed.iter().filter(|i| i.id == id);

        let filtered: Vec<&InstalledAppPackEntry> = match version {
            Some(v) => matches.filter(|i| i.version == v).collect(),
            None => matches.collect(),
        };

        match filtered.len() {
            0 => Err(anyhow!("AppPack (or version) is not installed")),
            1 => Ok(filtered[0].clone()),
            _ => Err(anyhow!(
                "Multiple versions installed — please specify a version"
            )),
        }
    }
}
