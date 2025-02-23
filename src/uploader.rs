use reqwest::header::HeaderMap;
use reqwest::blocking::{Client, Response, multipart::{Form, Part}};
use std::error::Error;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::{fs::{File, metadata}, io::{Read}, time, thread, sync::{Arc, Mutex}};
use std::path::Path;
use rayon::prelude::*;

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

#[derive(Debug, Default)]
pub struct Uploader {
    client: Client,
    url: String,
    headers: HeaderMap,
    kind_of_upload: KindOfUpload,
}

impl Uploader {
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::new(),
            url: url.to_string(),
            ..Default::default()
        }
    }

    pub fn new_multi_progress_bar() -> Arc<MultiProgress> {
        let mpb = Arc::new(MultiProgress::new());
        mpb
    }

    pub async fn upload_file(
        &self,
        path: &Path,
        pb: Arc<MultiProgress>,
        chunk_size: Option<usize>, // Option<usize> is a better choice than usize
    ) -> Result<Response, Box<dyn Error>> {
        let file_name = path
            .file_name()
            .ok_or("Failed to get file name")?
            .to_string_lossy()
            .to_string();

        let metadata = metadata(path)?;
        let file_size = metadata.len();

        let pb = pb.add(ProgressBar::new(file_size));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut request = self.client.post(&self.url);

        // Add form to request
        request = match self.kind_of_upload {
            KindOfUpload::Multipart => {
                let mut file_reader = File::open(path)?;
                let mut buffer = Vec::new();

                file_reader.read_to_end(&mut buffer)?;

                let request_body = Arc::new(Mutex::new(Vec::new()));
                let uploaded_size = Arc::new(Mutex::new(0u64));

                // Chunk size: 1 MiB
                let chunk_size = chunk_size.unwrap_or(64) * 1024;
                let  update_interval = time::Duration::from_millis(500);
                let mut last_update = Arc::new(Mutex::new(time::Instant::now()));

                buffer.par_chunks(chunk_size).for_each(|chunk| {
                    let mut request_body = request_body.lock().unwrap();
                    request_body.extend_from_slice(chunk);
                    let mut uploaded = uploaded_size.lock().unwrap();
                    *uploaded += chunk.len() as u64;
                    pb.set_position(*uploaded);

                    let mut last_update = last_update.lock().unwrap();

                    if last_update.elapsed() >= update_interval {
                        pb.tick();
                        *last_update = time::Instant::now();
                    }
                    thread::sleep(time::Duration::from_millis(100)); // ensure that the progress
                                                                     // bar is updated
                });

                // for chunk in buffer.chunks(chunk_size) {
                //     request_body.extend_from_slice(chunk);
                //     let mut uploaded = uploaded_size.lock().unwrap();
                //     *uploaded += chunk.len() as u64;
                //     pb.set_position(*uploaded);
                //     
                //     if last_update.elapsed() >= update_interval {
                //         pb.tick();
                //         last_update = time::Instant::now();
                //     }
                //     thread::sleep(time::Duration::from_millis(100)); // ensure that the progress
                //                                                      // bar is updated
                // }

                let part = Part::bytes(request_body.lock().unwrap().clone())
                    .file_name(file_name.clone())
                    .mime_str("application/octet-stream")?;
                // Create multipart form with file
                let form = Form::new().part(
                    "file",
                    part
                );
                request.multipart(form)
            }
            KindOfUpload::Binary => {
                // Create form with file
                let bytes = std::fs::read(path)?;

                request.body(bytes)
            }
        };

        // Add headers to request
        if !self.headers.is_empty() {
            request = request.headers(self.headers.clone());
        }

        // Send request
        let response = request.send()?;

        pb.finish_with_message("Upload complete: {file_name}");

        // Check if request was successful
        if response.status().is_success() {
            Ok(response)
        } else {
            Err(format!("Request failed with status: {}", response.status()).into())
        }
    }

    pub fn add_headers(&mut self, headers: HeaderMap) {
        self.headers = headers;
    }

    pub fn add_header(&mut self, key: String, value: String) -> Result<(), Box<dyn Error>> {
        self.headers.insert(
            reqwest::header::HeaderName::from_bytes(key.as_bytes())?,
            reqwest::header::HeaderValue::from_str(&value)?,
        );
        Ok(())
    }
}
