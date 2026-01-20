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

use crate::types::AppDesktopEntry;
use crate::types::AppSnapshotTriggerMode;
use anyhow::{Context, anyhow};
use serde::Deserialize;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use crate::utils::xdg_session_type_detector::get_freerdp_executable;

#[derive(Debug, Clone, Deserialize)]
pub struct AppBuildConfig {
    pub name: String,
    pub id: String,
    pub version: String,
    pub image: String,
    pub description: Option<String>,
    pub snapshot: AppSnapshotTriggerMode,
    pub readme: AppBuildConfigReadmeConfiguration,
    pub base_command: String,
    pub install_append: String,
    pub configure_append: String,
    pub configure_freerdp: String,
    pub desktop_entries: Option<Vec<AppDesktopEntry>>,
}

impl AppBuildConfig {
    pub fn get_boot_install_command(&self) -> Command {
        let full_command = format!("{} {}", self.base_command, self.install_append);
        let full_command = full_command.replace("$IMAGE_FILE_PATH", &self.image);

        println!("Full boot install {}", full_command);

        let full_command_args = full_command.split_whitespace().collect::<Vec<&str>>();
        let mut command = Command::new("qemu-system-x86_64");
        command.args(full_command_args);
        command
    }

    pub fn get_boot_configure_command(&self, rdp_port: u16) -> Command {
        let full_command = format!("{} {}", self.base_command, self.configure_append);
        let full_command = full_command.replace("$IMAGE_FILE_PATH", &self.image);
        let full_command = full_command.replace("$RDP_PORT", &rdp_port.to_string());

        println!("Full boot configure {}", full_command);

        let full_command_args = full_command.split_whitespace().collect::<Vec<&str>>();
        let mut command = Command::new("qemu-system-x86_64");
        command.args(full_command_args);
        command
    }

    pub fn get_rdp_configure_command(&self, rdp_port: u16) -> Command {
        let snap_real_home = std::env::var("SNAP_REAL_HOME").unwrap();
        let full_command = format!("{} /v:localhost:$RDP_PORT", self.configure_freerdp)
            .replace("$RDP_PORT", &rdp_port.to_string())
            .replace("$HOME", &snap_real_home);

        let full_command_args = full_command.split_whitespace().collect::<Vec<&str>>();
        let freerdp_exec = get_freerdp_executable();
        println!("Full {freerdp_exec} args {:?}", full_command_args);

        let mut command = Command::new(freerdp_exec);
        command.args(full_command_args);
        command
    }

    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let mut file = std::fs::File::open(path)
            .context(format!("Unable to open config file '{}'", path.display()))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .context("Unable to read config file contents")?;

        let cfg: Self = serde_yaml::from_slice(&buffer).context("Invalid YAML format in file")?;

        if !AppBuildConfig::is_valid_version(&cfg.version) {
            return Err(anyhow!("Invalid character in version: {}", cfg.version));
        }

        Ok(cfg)
    }

    pub fn is_valid_version(version: &str) -> bool {
        let forbidden_chars = [
            '/', '\\', ':', '*', '?', '"', '<', '>', '|', ' ', '&', ';', '`', '$',
        ];

        if version.chars().any(|c| forbidden_chars.contains(&c)) {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppBuildConfigReadmeConfiguration {
    #[serde(default = "default_readme_folder")]
    pub folder: String,
    #[allow(dead_code)]
    #[serde(default = "default_readme_index")]
    pub index: String,
}

fn default_readme_folder() -> String {
    "readme".to_string()
}

fn default_readme_index() -> String {
    "README.md".to_string()
}
