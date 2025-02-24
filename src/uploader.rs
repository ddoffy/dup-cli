use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::HeaderMap;
use reqwest::multipart;
use reqwest::{Body, Client, Response};
use std::error::Error;
use std::sync::Arc;
use std::{path::Path};
use tokio::fs::File;
use tokio::io::{AsyncRead, BufReader, AsyncReadExt};
use tokio::sync::Mutex;
use tokio_util::io::ReaderStream;
use crate::cli::{KindOfUpload};
use tokio::task;
use base64::{engine::general_purpose, Engine};
use serde_json::json;
use tokio::sync::Semaphore;

const MAX_CONCURRENT_UPLOADS: usize = 32;

struct ProgressReader<R> {
    inner: R,
    progress: Arc<Mutex<ProgressBar>>,
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressReader<R> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
        buf: &mut tokio::io::ReadBuf,
    ) -> std::task::Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let poll_result = std::pin::Pin::new(&mut self.inner).poll_read(cx, buf);
        if let std::task::Poll::Ready(Ok(())) = &poll_result {
            let after = buf.filled().len();
            let bytes_read = after - before;
            if bytes_read > 0 {
                let progress = self.progress.clone();
                tokio::spawn(async move {
                    let pb = progress.lock().await;
                    pb.inc(bytes_read as u64);
                });
            }
        }
        poll_result
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
            client: Client::builder().build().ok().unwrap(),
            url: url.to_string(),
            ..Default::default()
        }
    }

    pub async fn upload_file(
        self,
        path: &std::path::Path,
    ) -> Result<reqwest::Response, Box<dyn Error>> {
        let file_name = path
            .file_name()
            .ok_or("Failed to get file name")?
            .to_string_lossy()
            .to_string();

        let mut request = self.client.post(&self.url);

        // Add form to request
        request = match self.kind_of_upload {
            KindOfUpload::Multipart => {
                // Create multipart form with file
                let async_file = File::open(path).await?;
                let reader = BufReader::new(async_file);

                // convert the reader into a stream
                let stream = ReaderStream::new(reader);

                let body = Body::wrap_stream(stream);

                let part = multipart::Part::stream(body)
                    .file_name(file_name)
                    .mime_str("application/octet-stream")?;
                

                let form = multipart::Form::new().part("file", part);

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
        let response = request.send().await?;

        // Check if request was successful
        if response.status().is_success() {
            Ok(response)
        } else {
            Err(format!("Request failed with status: {}", response.status()).into())
        }
    }

    pub async fn upload_file_with_progress(self, path: &Path) -> Result<Response, Box<dyn Error>> {
        let file_name = path
            .file_name()
            .ok_or("Failed to get file name")?
            .to_string_lossy()
            .to_string();

        let file_size = std::fs::metadata(path)?.len();

        // create a progress bar
        let progress_bar = Arc::new(Mutex::new(ProgressBar::new(file_size)));
        {
            let pb = progress_bar.lock().await;
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})",
                    )
                    .unwrap()
                    .progress_chars("##-"),
            );
        }

        let mut request = self.client.post(&self.url);

        // Add form to request
        request = match self.kind_of_upload {
            KindOfUpload::Multipart => {
                // open file for async reading
                let async_file = File::open(path).await?;
                let reader = BufReader::new(async_file);

                // wrap the stream in a progress reader
                let progress_reader = ProgressReader {
                    inner: reader,
                    progress: progress_bar.clone(),
                };

                // convert async reader into a stream
                let stream =
                    ReaderStream::new(progress_reader);

                let body = Body::wrap_stream(stream);

                let part = multipart::Part::stream(body)
                    .file_name(file_name)
                    .mime_str("application/octet-stream")?;

                let form = multipart::Form::new().part("file", part);

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
        let response = request.send().await?;

        // finish progress bar
        {
            let pb = progress_bar.lock().await;
            let msg = format!("Uploaded {} bytes", pb.position());
            pb.finish_with_message(msg);
        }

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

    pub async fn upload_file_with_chunk_size(
        &self,
        path: &std::path::Path,
        chunk_size: usize,
    ) -> Result<(), Box<dyn Error>> {

        let file_metadata = std::fs::metadata(path)?;
        let file_size = file_metadata.len();
        let file_name = path
            .file_name()
            .ok_or("Failed to get file name")?
            .to_string_lossy()
            .to_string();

        println!("Uploading file {} with chunk size {}", file_name, chunk_size);

        let total_chunks = file_size.div_ceil(chunk_size as u64);

        let file = Arc::new(tokio::sync::Mutex::new(File::open(path).await?));
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_UPLOADS));

        let mut tasks = vec![];

        let client = self.client.clone();
        let url = self.url.clone();

        for chunk_id in 0..total_chunks {
            let filename_clone = file_name.clone();
            let client_clone = client.clone();
            let url_clone = url.clone();
            let file_clone = Arc::clone(&file);
            let semaphore_clone = Arc::clone(&semaphore);

            let task = task::spawn(async move {
                
                // limit concurrent uploads
                let _permit = semaphore_clone.acquire().await.unwrap();

                let mut file = file_clone.lock().await;
                let mut buffer = vec![0; chunk_size];

                // file.seek(SeekFrom::Start(chunk_id * chunk_size as u64)).await.unwrap();
                // read chunk manually using read_exact where possible
                let start_offset = chunk_id * chunk_size as u64;
                let read_pos = Arc::new(Mutex::new(start_offset));
                let mut bytes_read = 0;
                while bytes_read < chunk_size {
                    match file.read(&mut buffer[bytes_read..]).await {
                        Ok(0) => break,
                        Ok(n) => {
                            bytes_read += n;
                            let mut read_pos = read_pos.lock().await;
                            *read_pos += n as u64;
                        }
                        Err(e) => {
                            eprintln!("Error reading file: {:?}", e);
                            break;
                        }
                    }
                }

                buffer.truncate(bytes_read); // remove extra unused bytes
                 
                // Encode chunk in Base64 (required for JSON compatibility)
                let encoded_data = general_purpose::STANDARD.encode(&buffer);

                let payload = json!({
                    "filename": filename_clone,
                    "chunk_id": chunk_id,
                    "total_chunks": total_chunks,
                    "data": encoded_data,
                });


                let response = client_clone
                    .post(url_clone)
                    .json(&payload)
                    .send()
                    .await
                    .unwrap();

                println!("Chunk {} of {} uploaded: {:?}", chunk_id + 1, total_chunks, response.text().await.unwrap());
            });

            tasks.push(task);
        }

        for task in tasks {
            task.await.unwrap();
        }

        println!("File {} uploaded successfully", file_name);
        
        Ok(())
    }
}
