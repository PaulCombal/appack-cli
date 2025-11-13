use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstalledAppPackEntry {
    pub id: String,
    pub version: String,
    pub name: String,
    pub image: String,
    pub description: Option<String>,
    pub desktop_entries: Option<Vec<AppPackDesktopEntry>>,
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
            ));
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppPackIndexFile {
    pub name: String,
    pub id: String,
    pub version: String,
    pub image: String,
    pub description: Option<String>,
    pub snapshot: AppPackSnapshotMode,
    pub readme: ReadmeConfiguration,
    pub base_command: String,
    pub install_append: String,
    pub configure_append: String,
    pub freerdp_command: String,
    pub desktop_entries: Option<Vec<AppPackDesktopEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPackDesktopEntry {
    pub entry: String,
    pub icon: String,
    pub rdp_args: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppPackSnapshotMode {
    OnClose,
    Never,
    NeverLoad,
}

// Should we let the variables be replaced via the environment instead? Probably.
impl AppPackIndexFile {
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
        let full_command = self.freerdp_command.clone();
        let full_command = full_command.replace("$RDP_PORT", &rdp_port.to_string());
        let full_command = full_command.replace("$HOME", &snap_real_home);

        println!("Full RDP configure {}", full_command);

        let full_command_args = full_command.split_whitespace().collect::<Vec<&str>>();
        let mut command = Command::new("xfreerdp3");
        command.args(full_command_args);
        command
    }

    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let mut file = std::fs::File::open(path)
            .context(format!("Unable to open config file at {}", path.display()))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .context("Unable to read config file contents")?;

        let cfg: Self = serde_yaml::from_slice(&buffer).context("Invalid YAML format in file")?;
        let forbidden_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' ', '&', ';'];

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
