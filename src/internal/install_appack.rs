use crate::internal::types::{AppPackIndexFile, AppPackLocalSettings};
use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{PathBuf};
use virt::connect::Connect;
use virt::domain::Domain;
use virt::domain_snapshot::DomainSnapshot;
use zip::ZipArchive;

fn extract_config(archive: &mut ZipArchive<File>) -> Result<AppPackIndexFile> {
    let mut file = archive
        .by_name("AppPack.yaml")
        .map_err(|_| anyhow!("File 'AppPack.yaml' not found in archive"))?;

    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer)?;
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
            local_settings.save_folder
        ));
    }

    let image_filename = config.image.as_str();
    let image_fullpath = PathBuf::from(local_settings.images_folder.as_ref()).join(image_filename);

    if image_fullpath.exists() {
        return Err(anyhow!(
            "Image file already exists in {}: {image_filename}",
            local_settings.images_folder
        ));
    }

    println!("Extracting files (1/2)..");

    {
        let mut save_file = archive
            .by_name(save_filename)
            .map_err(|_| anyhow!("File '{}' not found in archive", save_filename))?;

        let mut outfile = File::create(&save_fullpath).map_err(|e| anyhow!("Unable to create file {}: {e}", save_fullpath.display()))?;
        io::copy(&mut save_file, &mut outfile)?;
    }

    println!("Extracting files (2/2)..");

    {
        let mut image_file = archive
            .by_name(image_filename)
            .map_err(|_| anyhow!("File '{}' not found in archive", image_filename))?;

        let mut outfile = File::create(&image_fullpath).map_err(|e| anyhow!("Unable to create file {}: {e}", save_fullpath.display()))?;
        io::copy(&mut image_file, &mut outfile)?;
    }

    Ok(())
}

fn create_domain(archive: &mut ZipArchive<File>, config: &AppPackIndexFile, settings: &AppPackLocalSettings) -> Result<()> {
    println!("Creating domain..");


    let mut file = archive
        .by_name("define.xml")
        .map_err(|_| anyhow!("File 'define.xml' not found in archive"))?;

    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer)?;
    let define_xml = String::from_utf8(buffer).map_err(|_| anyhow!("Invalid UTF-8 file: define.xml"))?;

    let conn = Connect::open(Some(&settings.qemu_uri))?;
    Domain::define_xml(&conn, &define_xml)?;

    let dom = Domain::lookup_by_name(&conn, config.domain.as_str())?;

    println!("Starting initializing domain..");

    dom.create()?;

    println!("Initializing application default state..");

    let snapshot_xml = r#"
      <domainsnapshot>
        <name>appack0</name>
        <description>App state on startup</description>
      </domainsnapshot>
    "#;

    DomainSnapshot::create_xml(&dom, snapshot_xml, 0)?;

    Ok(())
}

fn check_valid_app_pack(
    archive: &mut ZipArchive<File>,
    config: &AppPackIndexFile
) -> Result<()> {
    let required_files = [
        "define.xml",
        config.state.as_str(),
        config.image.as_str(),
    ];

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

pub fn install_appack(file_path: PathBuf, local_settings: AppPackLocalSettings) -> Result<()> {
    let file = File::open(&file_path)?;
    let mut archive = ZipArchive::new(file)?;

    // DEV ONLY
    // archive.file_names().for_each(|file| {
    //     println!("{}", file);
    // });

    let config = extract_config(&mut archive)?;
    check_valid_app_pack(&mut archive, &config)?;
    extract_files(&mut archive, &config, &local_settings)?;
    create_domain(&mut archive, &config, &local_settings)?;

    // Setup desktop integration

    Ok(())
}