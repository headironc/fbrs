use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Body, Client, Method, Url};
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

use crate::error::Error::{self, ProcessorNotFound};

#[derive(Debug)]
pub struct Processors {
    inner: HashMap<&'static str, Box<dyn Process>>,
}

#[async_trait]
pub trait Process: Debug + Send + Sync {
    async fn process(&self, io: IO) -> Result<(), Error>;
}

#[derive(Debug, Serialize)]
struct ProcessorError {
    message: String,
}

#[derive(Debug)]
pub struct NetworkIOProcessor {
    client: Client,
}

pub struct IO {
    inner: Inner,
}

enum Inner {
    NetworkIO(NetworkIO),
}

#[derive(Debug)]
struct NetworkIO {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Option<String>,
    timeout: Option<Duration>,
    result_path: PathBuf,
}

#[non_exhaustive]
#[derive(Debug, Deserialize)]
#[serde(tag = "processor_id")]
pub enum IOBuilder {
    #[serde(rename = "com.proxy.network.io")]
    NetworkIO(NetworkIOBuilder),
}

type Seconds = u64;

#[derive(Debug, Deserialize)]
pub struct NetworkIOBuilder {
    method: String,
    url: String,
    headers: Vec<HeaderBuilder>,
    body: Option<String>,
    timeout: Option<Seconds>,
    result_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct HeaderBuilder {
    name: String,
    value: String,
}

impl Processors {
    pub fn new(map: HashMap<&'static str, Box<dyn Process>>) -> Self {
        Self { inner: map }
    }

    pub fn add(&mut self, id: &'static str, processor: Box<dyn Process>) {
        self.inner.insert(id, processor);
    }

    pub fn get(&self, id: &'static str) -> Option<&dyn Process> {
        self.inner.get(id).map(|process| process.as_ref())
    }

    pub fn remove(&mut self, id: &'static str) -> Option<Box<dyn Process>> {
        self.inner.remove(id)
    }

    pub async fn process(&self, io: IO) -> Result<(), Error> {
        let processor_id = match io.inner {
            Inner::NetworkIO(_) => "com.proxy.network.io",
        };

        let processor = self
            .get(processor_id)
            .ok_or_else(|| ProcessorNotFound(processor_id.to_owned()))?;

        let result_path = match io.inner {
            Inner::NetworkIO(ref io) => io.result_path.to_owned(),
        };

        if let Err(error) = processor.process(io).await {
            error!("process error: {}", error);

            if let Some(parent) = result_path.parent() {
                if !parent.exists() {
                    create_dir_all(parent).await?;
                }
            }

            let mut file = File::create(result_path).await?;

            let error = ProcessorError {
                message: error.to_string(),
            };

            let bytes = to_vec(&error)?;

            file.write_all(&bytes).await?;
        }

        Ok(())
    }
}

impl Default for NetworkIOProcessor {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl Process for NetworkIOProcessor {
    async fn process(&self, io: IO) -> Result<(), Error> {
        let Inner::NetworkIO(io) = io.inner;

        let mut request_builder = self.client.request(io.method, io.url);

        request_builder = request_builder.headers(io.headers);

        if let Some(body) = io.body {
            request_builder = request_builder.body(Body::from(body));
        }

        if let Some(timeout) = io.timeout {
            request_builder = request_builder.timeout(timeout);
        }

        let request = request_builder.build()?;

        let response = self.client.execute(request).await?;

        info!("response: {:#?}", response);

        Ok(())
    }
}

impl IOBuilder {
    // 从json字节流中解析出一个 IOBuilder
    pub fn new(bytes: &[u8]) -> Result<Self, Error> {
        let builder = from_slice(bytes)?;

        Ok(builder)
    }

    pub fn build(self) -> Result<IO, Error> {
        let io = match self {
            IOBuilder::NetworkIO(builder) => Inner::NetworkIO(builder.build()?),
        };

        Ok(IO { inner: io })
    }
}

impl NetworkIOBuilder {
    fn build(self) -> Result<NetworkIO, Error> {
        let method = Method::from_str(&self.method)?;
        let url = Url::parse(&self.url)?;
        let headers = self.headers.into_iter().try_fold(
            HeaderMap::new(),
            |mut headers, header| -> Result<HeaderMap, Error> {
                let name = HeaderName::from_bytes(header.name.as_bytes())?;
                let value = HeaderValue::from_bytes(header.value.as_bytes())?;

                headers.insert(name, value);

                Ok(headers)
            },
        )?;

        let timeout = self.timeout.map(Duration::from_secs);

        Ok(NetworkIO {
            method,
            url,
            headers,
            body: self.body,
            timeout,
            result_path: self.result_path,
        })
    }
}
