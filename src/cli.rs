use clap::Parser;
use std::error::Error;
use std::io::{stdin, BufRead, IsTerminal};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Default)]
pub enum KindOfUpload {
    #[default]
    Multipart,
    Binary,
}

impl Clone for KindOfUpload {
    fn clone(&self) -> Self {
        match self {
            KindOfUpload::Multipart => KindOfUpload::Multipart,
            KindOfUpload::Binary => KindOfUpload::Binary,
        }
    }
}

impl std::str::FromStr for KindOfUpload {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "multipart" => Ok(KindOfUpload::Multipart),
            "binary" => Ok(KindOfUpload::Binary),
            _ => Err("Invalid kind of upload".into()),
        }
    }
}

#[derive(Debug, Parser, Default)]
#[clap(name = "Doffy uploader CLI", version = "0.1.7", author = "D. Doffy")]
#[clap(about = "Upload files to server parallelly", long_about = "Upload files to server parallelly, that supports multipart and binarry uploads, with progress bar")]
pub struct Cli {
    // host to upload to
    #[clap(short = 'H', long = "host", required = false, default_value = "")]
    pub host: String,
    #[clap(
        short = 'c',
        long = "category",
        default_value = "multipart",
        required = false
    )]
    pub category: KindOfUpload,
    // token to authenticate
    #[clap(short = 't', long = "token", required = false, default_value = "")]
    pub token: String,
    // paths to upload
    pub paths: Vec<std::path::PathBuf>,
    // show progress
    #[clap(short = 'p', long = "progress", required = false)]
    pub progress: bool,
    // chunk size
    #[clap(short = 's', long = "chunk-size", required = false)]
    pub chunk_size: Option<usize>,
}

impl Cli {
    pub fn from_args() -> Self {
        Self::parse()
    }

    pub fn validate(&mut self) -> Result<(), Box<dyn Error>> {
        if self.host.is_empty() {
            self.host = match std::env::var("UPLOAD_URL") {
                Ok(host) => host,
                Err(_) => {
                    return Err("No host provided. Please provide a host using the --host flag or UPLOAD_URL environment variable".into());
                }
            };
        }

        if self.paths.is_empty() {
            if stdin().is_terminal() {
                return Err("No files or directories provided".into());
            }

            self.paths = stdin()
                .lock()
                .lines()
                .map_while(|line| line.ok())
                .map(PathBuf::from)
                .collect();
        }

        Ok(())
    }
}
