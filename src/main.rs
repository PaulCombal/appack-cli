mod internal;

use crate::internal::install_appack::install_appack;
use crate::internal::types::AppPackLocalSettings;
use anyhow::Result;
use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, Debug, ValueEnum)]
enum CliAction {
    #[value(alias = "i")]
    Install,
    #[value(alias = "u")]
    Uninstall,
    Info,
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[arg(required = true, value_enum)]
    action: CliAction,

    #[arg(required_if_eq("action", "install"))]
    #[arg(required_if_eq("action", "i"))]
    file: Option<std::path::PathBuf>,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    dbg!("Args: {args:?}");

    let settings = AppPackLocalSettings::default();

    match args.action {
        CliAction::Install => {
            let file = args.file.unwrap();
            install_appack(file, settings)?
        }
        CliAction::Uninstall => {
            todo!()
        }
        CliAction::Info => {
            todo!()
        }
    }

    Ok(())
}
