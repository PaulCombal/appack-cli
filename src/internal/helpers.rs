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

use anyhow::{Context, Result, anyhow};
use std::net::{Ipv4Addr, TcpListener};
use std::path::Path;
use std::process::Command;

pub fn get_os_assigned_port() -> Result<u16> {
    let listener = TcpListener::bind(format!("{}:0", Ipv4Addr::LOCALHOST))?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

pub fn has_snapshot(snapshot_name: &str, image_name: &Path) -> Result<bool> {
    let output = Command::new("qemu-img")
        .arg("snapshot")
        .arg("-lU")
        .arg(image_name)
        .output()
        .context("Failed to get image snapshots")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to get image snapshots (output failed: {output:?})"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let contains_snapshot = stdout.contains(&format!(" {snapshot_name} "));

    Ok(contains_snapshot)
}
