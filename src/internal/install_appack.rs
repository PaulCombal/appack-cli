use crate::internal::installed_helpers::{get_installed, save_installed};
use crate::internal::types::{
    AppPackIndexFile, AppPackLocalSettings, InstalledAppPackEntry, InstalledAppPacks,
};
use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use zip::ZipArchive;

fn extract_config(archive: &mut ZipArchive<File>) -> Result<AppPackIndexFile> {
    let mut file = archive
        .by_name("AppPack.yaml")
        .map_err(|_| anyhow!("File 'AppPack.yaml' not found in archive"))?;

    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer)
        .map_err(|e| anyhow!("Unable to read config file: {e}"))?;
    serde_yaml::from_slice(&buffer).map_err(|e| anyhow!("Invalid YAML file: {e:?}"))
}

fn extract_files(
    archive: &mut ZipArchive<File>,
    config: &AppPackIndexFile,
    local_settings: &AppPackLocalSettings,
) -> Result<()> {
    let save_filename = config.state.as_str();
    let save_fullpath = PathBuf::from(local_settings.save_folder.as_ref()).join(save_filename);

    if save_fullpath.exists() {
        return Err(anyhow!(
            "Save file already exists in {}: {save_filename}",
            local_settings.save_folder.display()
        ));
    }

    let image_filename = config.image.as_str();
    let image_fullpath = PathBuf::from(local_settings.images_folder.as_ref()).join(image_filename);

    if image_fullpath.exists() {
        return Err(anyhow!(
            "Image file already exists in {}: {image_filename}",
            local_settings.images_folder.display()
        ));
    }

    println!("Extracting files (1/2)..");

    {
        let mut save_file = archive
            .by_name(save_filename)
            .map_err(|_| anyhow!("File '{}' not found in archive", save_filename))?;

        let mut outfile = File::create(&save_fullpath)
            .map_err(|e| anyhow!("Unable to create file {}: {e}", save_fullpath.display()))?;
        io::copy(&mut save_file, &mut outfile)?;
    }

    println!("Extracting files (2/2)..");

    {
        let mut image_file = archive
            .by_name(image_filename)
            .map_err(|_| anyhow!("File '{}' not found in archive", image_filename))?;

        let mut outfile = File::create(&image_fullpath)
            .map_err(|e| anyhow!("Unable to create file {}: {e}", save_fullpath.display()))?;
        io::copy(&mut image_file, &mut outfile)?;
    }

    Ok(())
}

fn check_valid_app_pack(
    archive: &mut ZipArchive<File>,
    config: &AppPackIndexFile,
    installed: &InstalledAppPacks,
) -> Result<()> {
    for entry in installed.installed.iter() {
        if entry.id == config.id {
            println!("Domain already installed: {}", entry.id);
            println!("Installed version: {}", entry.version);
            println!("File version: {}", config.version);
            return Err(anyhow!("Domain already exists"));
        }
    }

    let required_files = ["define.xml", config.state.as_str(), config.image.as_str()];

    // Collect all file names present in the archive into a HashSet
    let mut present_files = HashSet::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            present_files.insert(file.name().to_string());
        }
    }

    // Check which required files are missing
    let missing_files: Vec<_> = required_files
        .iter()
        .filter(|&&f| !present_files.contains(f))
        .collect();

    if !missing_files.is_empty() {
        return Err(anyhow!("Missing files: {:?}", missing_files));
    }

    Ok(())
}

fn add_and_save_installed(
    config: &AppPackIndexFile,
    settings: &AppPackLocalSettings,
    installed: &mut InstalledAppPacks,
) -> Result<()> {
    todo!();

    // let new_entry = InstalledAppPackEntry {
    //     id: config.id.clone(),
    //     description: config.description.clone(),
    //     version: config.version.clone(),
    //     name: config.name.clone(),
    //     image: config.image.clone(),
    //     desktop_entries: None, // TODO
    // };
    //
    // installed.installed.push(new_entry);
    //
    // save_installed(installed, settings)?;

    Ok(())
}

fn take_snapshot(path: &Path) -> Result<()> {
    let mut command = Command::new("qemu-img");
    command.arg("snapshot").arg("-c").arg("appack0").arg(path);

    let status = command
        .status()
        .map_err(|e| anyhow!("Unable to spawn qemu-img: {e}"));

    match status {
        Ok(_) => {}
        Err(e) => {
            return Err(anyhow!("Unable to spawn qemu-img: {e}"));
        }
    }

    Ok(())

    // builder
    //     .add_arg("-name", Some(format!("{},process={}", config.domain, config.domain)))
    //     .add_arg("-drive", Some(format!("file={},if=none,id=drive-virtio-disk0,format=qcow2,cache=none,discard=unmap", image_path.display())))
    //     .add_arg("-smp", Some("2,cores=2,threads=1,sockets=1".to_string())) // TODO
    //     .add_arg("-m", Some("4G".to_string())) // TODO
    //     .add_arg("-balloon", Some("virtio".to_string())) // TODO (uses-virtio-balloon or smth)
    // ;
}

pub fn install_appack(file_path: PathBuf, settings: AppPackLocalSettings) -> Result<()> {
    let file =
        File::open(&file_path).map_err(|e| anyhow!("Unable to open file {file_path:?}: {e}"))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| anyhow!("Unable to file as zip archive: {e}"))?;

    // DEV ONLY
    // archive.file_names().for_each(|file| {
    //     println!("{}", file);
    // });

    let config = extract_config(&mut archive)?;
    let mut installed = get_installed(&settings)?;

    check_valid_app_pack(&mut archive, &config, &installed)?;
    extract_files(&mut archive, &config, &settings)?;

    // 1. Create snapshot
    {
        let installed_image = PathBuf::from(settings.images_folder.as_ref()).join(config.image);
        take_snapshot(&installed_image)?;
    }

    // 2. Add to installed list

    return Ok(());

    // 2. Setup desktop integration

    Ok(())
}
