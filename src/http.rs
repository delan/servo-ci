use jane_eyre::eyre;
use reqwest::{
    IntoUrl,
    blocking::{Client, RequestBuilder},
};
use tracing::info;

pub trait ClientExt {
    fn logged_post(&self, url: impl IntoUrl) -> eyre::Result<RequestBuilder>;
    fn logged_get(&self, url: impl IntoUrl) -> eyre::Result<RequestBuilder>;
}

impl ClientExt for Client {
    fn logged_post(&self, url: impl IntoUrl) -> eyre::Result<RequestBuilder> {
        let url = url.into_url()?;
        info!("POST {url}");

        Ok(self.post(url))
    }

    fn logged_get(&self, url: impl IntoUrl) -> eyre::Result<RequestBuilder> {
        let url = url.into_url()?;
        info!("GET {url}");

        Ok(self.get(url))
    }
}
