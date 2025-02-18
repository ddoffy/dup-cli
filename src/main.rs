use reqwest::multipart;
use std::error::Error;

#[tokio::main]
async fn main() {
    let args = Cli::from_args();

    // read Url from environment variable
    let url = match std::env::var("UPLOAD_URL") {
        Ok(url) => url,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    let mut paths = vec![];
    // print full path of each file
    for path in args.path {
        match std::fs::canonicalize(&path) {
            Ok(full_path) => {
                handle_path(full_path, &mut paths);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    let mut handles = vec![];
    for path in paths {
        let url_clone = url.clone();
        let handle = tokio::spawn(async move {
            handle_upload_file(path, &url_clone).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
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

async fn handle_upload_file(path: std::path::PathBuf, url: &str) {
    let uploader = Uploader::new(url);
    println!("Starting upload of {}", path.display());

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

#[derive(Debug)]
struct Cli {
    path: Vec<std::path::PathBuf>,
}

impl Cli {
    fn from_args() -> Self {
        let mut path = Vec::new();
        for arg in std::env::args().skip(1) {
            path.push(std::path::PathBuf::from(arg));
        }
        Self { path }
    }
}

struct Uploader {
    client: reqwest::Client,
    url: String,
}

impl Uploader {
    fn new(url: &str) -> Self {
        Self {
            client: reqwest::Client::builder().build().ok().unwrap(),
            url: url.to_string(),
        }
    }

    async fn upload_file(&self, path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        let file_name = path
            .file_name()
            .ok_or("Failed to get file name")?
            .to_string_lossy()
            .to_string();

        // Create multipart form with file
        let form = multipart::Form::new().part(
            "file",
            multipart::Part::bytes(std::fs::read(path)?).file_name(file_name),
        );

        let request = self.client.post(&self.url).multipart(form);

        // Send request
        let response = request.send().await?;

        // Check if request was successful
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Request failed with status: {}", response.status()).into())
        }
    }
}
