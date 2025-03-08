use clap::{arg, command, Parser};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Destination phone number with country code (without '+'). Example: 48601222333
    #[arg(short = 'n', long = "to")]
    pub phone_number: String,

    /// Message content. Only ascii characters are supported. Maximum 140 characters.
    #[arg(short, long)]
    pub message: String,

    /// SMSC host address with port. [Default value: 192.168.254.36:3600]"
    #[arg(long)]
    pub server: Option<String>,

    /// SMSC server system_id
    #[arg(long)]
    pub login: Option<String>,

    /// SMSC server password
    #[arg(long)]
    pub password: Option<String>,

    /// Address of your gateway, can be a number like 444 or text "MY COMPANY"
    #[arg(long)]
    pub source_addr: Option<String>,

    /// Disables ansii output. Useful on terminals that does not support it.
    #[arg(long)]
    pub disable_ansii: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub login: Option<String>,
    pub password: Option<String>,
    pub smsc_host: Option<String>,
    pub source_addr: Option<String>,
}

impl Config {
    pub fn load() -> Option<Self> {
        let paths = [
            PathBuf::from("config.yml"),
            user_config_path(),
            machine_config_path(),
        ];

        let existing_paths = paths.iter().filter(|p| p.exists());

        tracing::debug!("{:?}", std::env::current_exe());
        tracing::debug!("{:?}", existing_paths);

        for path in paths.iter().filter(|p| p.exists()) {
            if let Ok(contents) = fs::read_to_string(path) {
                match serde_yaml::from_str::<Config>(&contents) {
                    Ok(c) => return Some(c),

                    Err(e) => tracing::error!("Fucked {}", e),
                }
            } else {
                tracing::error!("Fucked")
            }
        }

        None
    }
}

fn user_config_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").unwrap()).join("smscli\\config.yml")
    } else {
        config_dir().unwrap().join("smscli/config.yml")
    }
}

fn machine_config_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\ProgramData\smscli\config.yml")
    } else {
        PathBuf::from("/etc/smscli/config.yml")
    }
}
