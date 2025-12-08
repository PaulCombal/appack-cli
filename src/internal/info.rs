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

use crate::types::app_installed::InstalledAppPackEntry;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub fn print_info(file: &Path) -> anyhow::Result<()> {
    const TARGET_FILE: &str = "AppPack.yaml";

    let zip_file = File::open(file)?;
    let mut archive = ZipArchive::new(zip_file)?;

    // 2. Find and open the file named "AppPack.yaml" inside the archive
    let mut packed_file = archive.by_name(TARGET_FILE).map_err(|_| {
        anyhow::anyhow!(
            "File '{}' not found in zip archive: {}",
            TARGET_FILE,
            file.display()
        )
    })?;

    // 3. Read the content of the file into a String
    let mut contents = String::new();
    packed_file.read_to_string(&mut contents)?;

    // 4. Unserialize the YAML content with serde_yaml
    let info: InstalledAppPackEntry = serde_yaml::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize '{}': {}", TARGET_FILE, e))?;

    // 5. Print the deserialized information
    println!(
        "Successfully read info from '{}' in {}:",
        TARGET_FILE,
        file.display()
    );
    println!("{:#?}", info);

    Ok(())
}
