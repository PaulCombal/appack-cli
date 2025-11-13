mod internal;
mod logger;

use crate::internal::creator::{
    creator_boot, creator_boot_install, creator_new, creator_pack, creator_snapshot,
};
use crate::internal::info::print_info;
use crate::internal::install_appack::install_appack;
use crate::internal::launch::launch;
use crate::internal::list_installed::list_installed;
use crate::internal::reset::reset;
use crate::internal::types::AppPackLocalSettings;
use crate::internal::uninstall_appack::uninstall_appack;
use crate::logger::logger::log_debug;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    action: CliAction,
}

#[derive(Debug, Subcommand)]
enum CliAction {
    #[clap(alias = "i")]
    Install {
        file: PathBuf,
    },

    #[clap(alias = "u")]
    Uninstall {
        id: String,
    },

    Creator {
        action: CliCreatorAction,
    },

    #[clap(alias = "li")]
    ListInstalled,

    Launch {
        id: String,
        rdp_args: Option<String>,
        #[clap(long)]
        version: Option<String>,
    },

    Reset {
        id: String,
        #[clap(long)]
        version: Option<String>,
    },

    Info,
}

#[derive(Debug, Subcommand, ValueEnum, Clone)]
enum CliCreatorAction {
    New,
    Boot,
    BootInstall,
    Snapshot,
    Pack,
}

fn main() -> Result<()> {
    log_debug("AppPack starting");

    let args = match Cli::try_parse() {
        Ok(args) => args,
        Err(e) => {
            log_debug("Error parsing args:");
            log_debug(&e);
            return Err(anyhow::anyhow!(e));
        }
    };

    let settings = AppPackLocalSettings::default();

    match args.action {
        CliAction::Install { file } => install_appack(file, settings)?,
        CliAction::Uninstall { id } => uninstall_appack(&settings, &id)?,
        CliAction::Creator { action } => match action {
            CliCreatorAction::New => {
                creator_new()?;
            }
            CliCreatorAction::BootInstall => {
                creator_boot_install()?;
            }
            CliCreatorAction::Boot => {
                creator_boot()?;
            }
            CliCreatorAction::Snapshot => {
                creator_snapshot()?;
            }
            CliCreatorAction::Pack => {
                creator_pack()?;
            }
        },
        CliAction::ListInstalled => {
            list_installed(settings)?;
        }
        CliAction::Info => {
            print_info(&settings);
        }
        CliAction::Launch {
            id,
            version,
            rdp_args,
        } => match launch(&settings, id, version.as_deref(), rdp_args.as_deref()) {
            Ok(_) => {}
            Err(e) => {
                log_debug("Error launching app pack:");
                log_debug(&e);
                return Err(anyhow::anyhow!(e));
            }
        },
        CliAction::Reset { id, version } => {
            reset(&settings, id, version.as_deref())?;
        }
    }

    log_debug("AppPack stopped");

    Ok(())
}
