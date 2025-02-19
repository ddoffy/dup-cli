use crate::uploader::Uploader;
use crate::cli::Cli;

pub mod cli;
pub mod uploader;

#[tokio::main]
async fn main() {
    let mut args = Cli::from_args();

    if let Err(err) = args.validate() {
        eprintln!("Error: {}", err);
        ::std::process::exit(1);
    }

    let mut paths = vec![];
    // print full path of each file
    for path in args.paths {
        match std::fs::canonicalize(&path) {
            Ok(full_path) => {
                handle_path(full_path, &mut paths);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    let mut handles = vec![];

    for path in paths {
        let url_clone = args.host.clone();
        let category = args.category.clone();
        let handle = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                handle_upload_file(path, &url_clone, category).await;
            });
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn handle_path(path: std::path::PathBuf, paths: &mut Vec<std::path::PathBuf>) {
    if path.is_dir() {
        handle_dir(path, paths);
    } else if path.is_file() {
        paths.push(path.clone());
    } else {
        println!("{} is not a file or directory", path.display());
    }
}

async fn handle_upload_file(path: std::path::PathBuf, url: &str, kind_of_upload: uploader::KindOfUpload) {
    let mut uploader = Uploader::new(url);
    println!("Starting upload of {}", path.display());

    if kind_of_upload == uploader::KindOfUpload::Binary{
        match uploader.add_header("Content-Type".to_string(), "application/octet-stream".to_string()) {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
        match uploader.add_header("X-Filename".to_string(), path.file_name().unwrap().to_str().unwrap().to_string()) {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    match uploader.upload_file(&path).await {
        Ok(_) => println!("Upload of {} successful", path.display()),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn handle_dir(path: std::path::PathBuf, paths: &mut Vec<std::path::PathBuf>) {
    println!("{} is a directory", path.display());
    match std::fs::read_dir(&path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        handle_path(path, paths);
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

