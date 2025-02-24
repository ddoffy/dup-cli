use crate::cli::Cli;
use crate::uploader::Uploader;

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

    // create a channel to sum total file size
    let (tx, rx) = std::sync::mpsc::channel();

    let mut handles = vec![];

    let total_time = std::time::Instant::now();

    if args.progress {
        for path in paths {
            let url_clone = args.host.clone();
            let category = args.category.clone();
            let tx_clone = tx.clone();
            let _ = handle_upload_file_with_progress(path, &url_clone, category, tx_clone).await;
        }
    } else if let Some(chunk_size) = args.chunk_size {
        for path in paths {
            let url_clone = args.host.clone();
            let tx_clone = tx.clone();
            let handle = std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().unwrap();

                runtime.block_on(async {
                    let _ = handle_upload_file_with_chunk_size(path, &url_clone, tx_clone, chunk_size).await;
                });
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    } else {
        for path in paths {
            let url_clone = args.host.clone();
            let category = args.category.clone();
            let tx_clone = tx.clone();
            let handle = std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().unwrap();

                runtime.block_on(async {
                    let _ = handle_upload_file(path, &url_clone, category, tx_clone).await;
                });
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    let total_time = total_time.elapsed().as_secs();

    println!("Total time: {}s", total_time);

    let mut total_size = 0;
    // drain the channel to sum total file size
    while let Ok(size) = rx.try_recv() {
        total_size += size;
    }

    println!("Total size: {}", file_size_human_readable(total_size));

    // the average speed of the upload
    let average_speed = total_size as f64 / total_time as f64;
    println!(
        "Average speed: {}/s",
        file_size_human_readable(average_speed as u64)
    );
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

fn file_size_human_readable(file_size: u64) -> String {
    if file_size <= 1024 {
        format!("{}B", file_size)
    } else if file_size <= 1024 * 1024 {
        format!("{:.2}KB", file_size as f64 / 1024.0)
    } else if file_size <= 1024 * 1024 * 1024 {
        format!("{:.2}MB", file_size as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.2}GB", file_size as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

async fn handle_upload_file(
    path: std::path::PathBuf,
    url: &str,
    kind_of_upload: cli::KindOfUpload,
    tx: std::sync::mpsc::Sender<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let time = std::time::Instant::now();
    let mut uploader = Uploader::new(url);
    let file_size = std::fs::metadata(&path)?.len();
    tx.send(file_size).unwrap();
    let file_size = file_size_human_readable(file_size);

    println!("Starting upload of {} [{}]", path.display(), file_size);

    if kind_of_upload == cli::KindOfUpload::Binary {
        match uploader.add_header(
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        ) {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
        match uploader.add_header(
            "X-Filename".to_string(),
            path.file_name().unwrap().to_str().unwrap().to_string(),
        ) {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    match uploader.upload_file(&path).await {
        Ok(res) => {
            let download_link = match res.text().await {
                Ok(text) => text,
                Err(_) => "Error".to_string(),
            };
            println!(
                "[{}s][{}][{}] - Download: {}",
                time.elapsed().as_secs(),
                path.display(),
                file_size,
                download_link
            );
            // check if res is json, text print it
        }
        Err(e) => {
            eprintln!("[{}s] Error: {}", time.elapsed().as_secs(), e);
            return Err(e);
        }
    };

    Ok(())
}

async fn handle_upload_file_with_progress(
    path: std::path::PathBuf,
    url: &str,
    kind_of_upload: cli::KindOfUpload,
    tx: std::sync::mpsc::Sender<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let time = std::time::Instant::now();
    let mut uploader = Uploader::new(url);
    let file_size = std::fs::metadata(&path)?.len();
    tx.send(file_size).unwrap();
    let file_size = file_size_human_readable(file_size);

    println!("Starting upload of {} [{}]", path.display(), file_size);

    if kind_of_upload == cli::KindOfUpload::Binary {
        match uploader.add_header(
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        ) {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
        match uploader.add_header(
            "X-Filename".to_string(),
            path.file_name().unwrap().to_str().unwrap().to_string(),
        ) {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    match uploader.upload_file_with_progress(&path).await {
        Ok(res) => {
            let download_link = match res.text().await {
                Ok(text) => text,
                Err(_) => "Error".to_string(),
            };
            println!(
                "[{}s][{}][{}] - Download: {}",
                time.elapsed().as_secs(),
                path.display(),
                file_size,
                download_link
            );
            // check if res is json, text print it
        }
        Err(e) => {
            eprintln!("[{}s] Error: {}", time.elapsed().as_secs(), e);
            return Err(e);
        }
    };

    Ok(())
}

async fn handle_upload_file_with_chunk_size(
    path: std::path::PathBuf,
    url: &str,
    tx: std::sync::mpsc::Sender<u64>,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let time = std::time::Instant::now();
    let uploader = Uploader::new(url);
    let file_size = std::fs::metadata(&path)?.len();
    tx.send(file_size).unwrap();
    let file_size = file_size_human_readable(file_size);

    println!("Starting upload of {} [{}]", path.display(), file_size);

    match uploader.upload_file_with_chunk_size(&path, chunk_size).await {
        Ok(_) => {
            // let download_link = match res.text().await {
            //     Ok(text) => text,
            //     Err(_) => "Error".to_string(),
            // };
            println!(
                "[{}s][{}][{}]",
                time.elapsed().as_secs(),
                path.display(),
                file_size
            );
            // check if res is json, text print it
        }
        Err(e) => {
            eprintln!("[{}s] Error: {}", time.elapsed().as_secs(), e);
            return Err(e);
        }
    };

    Ok(())
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
