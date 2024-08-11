use async_trait::async_trait;
use reqwest::header::HeaderMap;
use reqwest::{Client, Error, Response};
use std::time::Duration;

#[async_trait]
pub trait HttpClient {
    async fn get(&self, url: &str, headers: HeaderMap) -> Result<Response, Error>;
    async fn post(&self, url: &str, headers: HeaderMap, body: String) -> Result<Response, Error>;
}

pub struct SystemHttpClient {
    client: Client,
}

impl SystemHttpClient {
    pub fn new(timeout: u64) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()?;

        Ok(Self { client })
    }
}

#[async_trait]
impl HttpClient for SystemHttpClient {
    async fn get(&self, url: &str, headers: HeaderMap) -> Result<Response, Error> {
        self.client.get(url).headers(headers).send().await
    }

    async fn post(&self, url: &str, headers: HeaderMap, body: String) -> Result<Response, Error> {
        self.client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await
    }
}
