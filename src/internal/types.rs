use std::io::Read;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::anyhow;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstalledAppPackEntry {
    pub id: String,
    pub version: String,
    pub name: String,
    pub image: String,
    pub description: Option<String>,
    pub desktop_entries: Option<Vec<String>>,
    pub snapshot_mode: AppPackSnapshotMode,
    pub qemu_command: String,
    pub freerdp_command: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InstalledAppPacks {
    #[serde(default)]
    pub installed: Vec<InstalledAppPackEntry>,
}

#[derive(Debug)]
pub struct AppPackLocalSettings {
    pub installed_file: PathBuf,
    pub home_dir: PathBuf,
    pub desktop_entries_dir: PathBuf,
}

impl From<AppPackIndexFile> for InstalledAppPackEntry {
    fn from(value: AppPackIndexFile) -> Self {
        Self {
            id: value.id,
            version: value.version,
            image: value.image,
            name: value.name,
            description: value.description,
            desktop_entries: None,
            qemu_command: format!("{} {}", value.base_command, value.configure_append),
            freerdp_command: value.freerdp_command,
            snapshot_mode: value.snapshot,
        }
    }
}

impl Default for AppPackLocalSettings {
    fn default() -> Self {
        let snap_home = std::env::var("SNAP_USER_COMMON").unwrap();
        let snap_home = PathBuf::from(snap_home);
        let user_real_home = std::env::var("SNAP_REAL_HOME").unwrap();
        let user_real_home = PathBuf::from(user_real_home);
        Self {
            home_dir: snap_home.clone(),
            installed_file: snap_home.join("installed.yaml"),
            desktop_entries_dir: user_real_home.join(".local").join("share").join("applications"),
        }
    }
}

impl AppPackLocalSettings {
    pub fn check_ok(&self) -> anyhow::Result<()> {
        if !self.home_dir.exists() {
            return Err(anyhow!("Home directory does not exist: {}", self.home_dir.display()));
        }

        if !self.desktop_entries_dir.exists() {
            return Err(anyhow!("Desktop entries directory does not exist: {}", self.desktop_entries_dir.display()));
        }

        Ok(())
    }

    pub fn get_installed(&self) -> anyhow::Result<InstalledAppPacks> {
        let installed_filepath = self.installed_file.clone();

        let installed_app_packs: InstalledAppPacks = if installed_filepath.exists() {
            let content = std::fs::read_to_string(&installed_filepath).map_err(|e| {
                anyhow!(
                "Failed to read installed file {}: {}",
                installed_filepath.display(),
                e
            )
            })?;
            serde_yaml::from_str(&content).map_err(|e| {
                anyhow!(
                "Failed to parse installed file {}: {}",
                installed_filepath.display(),
                e
            )
            })?
        } else {
            InstalledAppPacks {
                installed: Vec::new(),
            }
        };

        Ok(installed_app_packs)
    }

    pub fn save_installed(
        &self,
        installed_app_packs: InstalledAppPacks,
    ) -> anyhow::Result<()> {
        let installed_filepath = self.installed_file.clone();
        let content = serde_yaml::to_string(&installed_app_packs)
            .map_err(|e| anyhow!("Failed to serialize installed app packs: {}", e))?;
        std::fs::write(&installed_filepath, content).map_err(|e| {
            anyhow!(
            "Failed to write installed file {}: {}",
            installed_filepath.display(),
            e
        )
        })?;

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppPackIndexFile {
    pub name: String,
    pub id: String,
    pub version: String,
    pub state: Option<String>,
    pub image: String,
    pub description: Option<String>,
    pub snapshot: AppPackSnapshotMode,
    pub readme: ReadmeConfiguration,
    pub base_command: String,
    pub install_append: String,
    pub configure_append: String,
    pub freerdp_command: String,
    pub desktop_entries: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppPackSnapshotMode {
    OnClose,
    Never,
}

// Should we let the variables be replaced via the environment instead? Probably.
impl AppPackIndexFile {
    pub fn get_boot_install_command(&self) -> Command {
        let full_command = format!("{} {}", self.base_command, self.install_append);
        let full_command = full_command.replace("$IMAGE_FILE_PATH", &self.image);

        println!("Full boot install {}", full_command);

        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg(format!("qemu-system-x86_64 {}", full_command));
        command
    }

    pub fn get_boot_configure_command(&self, rdp_port: u16) -> Command {
        let full_command = format!("{} {}", self.base_command, self.configure_append);
        let full_command = full_command.replace("$IMAGE_FILE_PATH", &self.image);
        let full_command = full_command.replace("$RDP_PORT", &rdp_port.to_string());

        println!("Full boot configure {}", full_command);

        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg(format!("qemu-system-x86_64 {}", full_command));
        command
    }

    pub fn get_rdp_configure_command(&self, rdp_port: u16) -> Command {
        let full_command = self.freerdp_command.clone();
        let full_command = full_command.replace("$RDP_PORT", &rdp_port.to_string());

        println!("Full RDP configure {}", full_command);

        let mut command = Command::new("bash");
        command.arg("-c").arg(format!("xfreerdp3 {}", full_command));
        command
    }

    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let mut file = std::fs::File::open(path)
            .map_err(|e| anyhow!("Unable to open config file at {}: {}", path.display(), e))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| anyhow!("Unable to read config file contents: {}", e))?;

        let cfg: Self = serde_yaml::from_slice(&buffer).map_err(|e| anyhow!("Invalid YAML format in file: {:?}", e))?;
        let forbidden_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '];

        if cfg.version.chars().any(|c| forbidden_chars.contains(&c)) {
            return Err(anyhow!("Invalid character in version: {}", cfg.version));
        }

        Ok(cfg)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadmeConfiguration {
    #[serde(default = "default_readme_folder")]
    pub folder: String,
    #[serde(default = "default_readme_index")]
    pub index: String,
}

fn default_readme_folder() -> String {
    "readme".to_string()
}

fn default_readme_index() -> String {
    "README.md".to_string()
}
