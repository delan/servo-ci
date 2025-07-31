use jane_eyre::eyre::{self, bail};
use reqwest::{
    blocking::{Client, ClientBuilder, RequestBuilder},
    header::{ACCEPT, AUTHORIZATION, HeaderName, HeaderValue, USER_AGENT},
};

pub struct GithubApi(Client);

impl GithubApi {
    pub fn client(github_token: impl AsRef<str>) -> eyre::Result<Self> {
        // <https://docs.github.com/en/rest/about-the-rest-api/api-versions?apiVersion=2022-11-28>
        // <https://docs.github.com/en/rest/using-the-rest-api/getting-started-with-the-rest-api?apiVersion=2022-11-28#user-agent>
        let authorization = format!("Bearer {}", github_token.as_ref());
        let headers = [
            (
                ACCEPT,
                HeaderValue::from_str("application/vnd.github+json")?,
            ),
            (AUTHORIZATION, HeaderValue::from_str(&authorization)?),
            (
                HeaderName::from_static("x-github-api-version"),
                HeaderValue::from_static("2022-11-28"),
            ),
            (
                USER_AGENT,
                HeaderValue::from_static("ServoCI/0 (<https://github.com/servo/ci>)"),
            ),
        ];
        let client = ClientBuilder::new()
            .default_headers(headers.into_iter().collect())
            .build()?;

        Ok(Self(client))
    }

    pub fn post(&self, path: impl AsRef<str>) -> eyre::Result<RequestBuilder> {
        Ok(self.0.post(Self::compute_url(path)?))
    }

    pub fn get(&self, path: impl AsRef<str>) -> eyre::Result<RequestBuilder> {
        Ok(self.0.get(Self::compute_url(path)?))
    }

    fn compute_url(path: impl AsRef<str>) -> eyre::Result<String> {
        let path = path.as_ref();
        let Some(path) = path.strip_prefix("/") else {
            bail!("Path does not start with slash: {path}");
        };

        Ok(format!("https://api.github.com/{path}"))
    }
}
