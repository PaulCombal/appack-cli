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

use anyhow::{Context, anyhow};
use std::fs::File;
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub fn zip_dir(
    zip: &mut ZipWriter<File>,
    zip_options: &SimpleFileOptions,
    dirpath: &Path,
) -> anyhow::Result<()> {
    let root_dir_name = dirpath
        .file_name()
        .ok_or_else(|| anyhow!("Invalid directory path"))?
        .to_str()
        .ok_or_else(|| anyhow!("Directory name contains invalid UTF-8"))?;

    zip_dir_recursive(zip, zip_options, dirpath, Path::new(root_dir_name))?;

    let dir_name_in_zip = format!("{}/", root_dir_name);
    zip.add_directory(&dir_name_in_zip, *zip_options)?;

    Ok(())
}

fn zip_dir_recursive(
    zip: &mut ZipWriter<File>,
    zip_options: &SimpleFileOptions,
    current_path: &Path,
    path_in_zip_prefix: &Path,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();

        let name = entry.file_name();
        let path_in_zip = path_in_zip_prefix.join(name);
        let path_in_zip_str = path_in_zip
            .to_str()
            .ok_or_else(|| anyhow!("Path contains invalid UTF-8: {:?}", path))?;

        if path.is_dir() {
            let dir_name_in_zip = format!("{}/", path_in_zip_str);
            zip.add_directory(&dir_name_in_zip, *zip_options)
                .context("Failed to add directory to zip")?;

            zip_dir_recursive(zip, zip_options, &path, &path_in_zip)?;
        } else if path.is_file() {
            zip.start_file(path_in_zip_str, *zip_options)
                .context("Failed to start file in zip")?;

            let mut f = File::open(&path).context(format!("Failed to open file {path:?}"))?;

            std::io::copy(&mut f, zip).context(format!("Failed to copy file {path:?} to zip"))?;
        }
    }

    Ok(())
}
