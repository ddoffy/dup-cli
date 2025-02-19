use reqwest::header::HeaderMap;
use reqwest::multipart;
use reqwest::Client;
use std::error::Error;

#[derive(Debug, PartialEq)]
pub enum KindOfUpload {
    Multipart,
    Binary,
}

impl Default for KindOfUpload {
    fn default() -> Self {
        KindOfUpload::Multipart
    }
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
            client: Client::builder().build().ok().unwrap(),
            url: url.to_string(),
            ..Default::default()
        }
    }

    pub async fn upload_file(self, path: &std::path::Path) -> Result<(), Box<dyn Error>> {
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
                let form = multipart::Form::new().part(
                    "file",
                    multipart::Part::bytes(std::fs::read(path)?).file_name(file_name),
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
        let response = request.send().await?;

        // Check if request was successful
        if response.status().is_success() {
            Ok(())
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
