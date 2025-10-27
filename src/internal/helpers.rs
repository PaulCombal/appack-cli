use anyhow::{Result, anyhow};
use std::fs::File;
use std::path::Path;
use zip::ZipWriter;
use zip::write::{SimpleFileOptions, StreamWriter};

pub fn zip_dir(
    zip: &mut ZipWriter<StreamWriter<File>>,
    zip_options: &SimpleFileOptions,
    dirpath: &Path,
) -> Result<()> {
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
    zip: &mut ZipWriter<StreamWriter<File>>,
    zip_options: &SimpleFileOptions,
    current_path: &Path,
    path_in_zip_prefix: &Path,
) -> Result<()> {
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
                .map_err(|e| anyhow!("Failed to add directory to zip: {}", e))?;

            zip_dir_recursive(zip, zip_options, &path, &path_in_zip)?;
        } else if path.is_file() {
            zip.start_file(path_in_zip_str, *zip_options)
                .map_err(|e| anyhow!("Failed to start file in zip: {}", e))?;

            let mut f =
                File::open(&path).map_err(|e| anyhow!("Failed to open file {:?}: {}", path, e))?;

            std::io::copy(&mut f, zip)
                .map_err(|e| anyhow!("Failed to copy file {:?} to zip: {}", path, e))?;
        }
    }

    Ok(())
}
