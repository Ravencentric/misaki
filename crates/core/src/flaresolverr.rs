use crate::UrlStatus;
use anyhow::{Context, Result};
use reqwest::{Url, header::CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Represents a command to be sent to the FlareSolverr service.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum FalreSolverrCommand {
    /// Represents a GET request to be sent to a specified URL using FlareSolverr.
    Get {
        /// The URL to which the GET request will be sent.
        url: String,
        /// The maximum timeout in seconds for the request.
        timeout: u32,
        /// The session identifier to be used for the request.
        session: Uuid,
    },

    /// Creates a new session with the given UUID.
    CreateSession(
        /// The unique identifier for the session to be created.
        Uuid,
    ),

    /// Destroys an existing session with the given UUID.
    DestroySession(
        /// The unique identifier for the session to be destroyed.
        Uuid,
    ),
}

impl FalreSolverrCommand {
    /// Converts a `FalreSolverrCommand` instance into a `serde_json::Value` representing
    /// the corresponding JSON command structure for FlareSolverr.
    ///
    /// This JSON representation can be directly POSTed to the FlareSolverr API.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            FalreSolverrCommand::Get {
                url,
                timeout,
                session,
            } => serde_json::json!({
                "cmd": "request.get",
                "url": &url,
                "maxTimeout": timeout * 1000, // maxTimeout accepts milliseconds
                "session": &session
            }),
            FalreSolverrCommand::CreateSession(session) => serde_json::json!({
                "cmd": "sessions.create",
                "session": &session,
            }),
            FalreSolverrCommand::DestroySession(session) => serde_json::json!({
                "cmd": "sessions.destroy",
                "session": &session,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FlareSolverrSolution {
    url: String,
    status: Option<u16>,
}

/// Represents the response structure returned by the FlareSolverr service.
/// This struct is solely used for deserializing the JSON response from the
/// `response.json()` call.
#[derive(Serialize, Deserialize, Debug)]
struct FlareSolverrResponse {
    solution: FlareSolverrSolution,
}

impl From<FlareSolverrResponse> for UrlStatus {
    fn from(response: FlareSolverrResponse) -> Self {
        UrlStatus {
            url: response.solution.url,
            status: response.solution.status,
        }
    }
}

/// Client for interacting with the FlareSolverr service, managing session lifecycle and requests.
///
/// # Session Management
///
/// Each instance of `FlareSolverrClient` establishes a new session with the FlareSolverr service.
/// Sessions must be explicitly destroyed using [`close`](Self::close) when no longer needed.
/// Accumulating too many active sessions can degrade FlareSolverr performance.
///
/// # Example
///
/// ```
/// let client = FlareSolverrClient::new(reqwest::Client::new(), 60, "http://localhost:8191").await?;
/// let status = client.check("https://example.com").await;
/// client.close().await?;
/// ```
#[derive(Clone, Debug)]
pub(crate) struct FlareSolverrClient {
    client: reqwest::Client,
    session: Uuid,
    timeout: u32,
    url: Url,
}

impl FlareSolverrClient {
    /// Creates a new `FlareSolverrClient` instance, establishing a session with FlareSolverr.
    ///
    /// # Arguments
    ///
    /// * `client` - The underlying [`reqwest::Client`] used for HTTP requests.
    /// * `timeout` - The maximum timeout in seconds for each request.
    /// * `url` - The FlareSolverr service endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the session could not be created or if the FlareSolverr service is unreachable.
    pub(crate) async fn new(
        client: reqwest::Client,
        timeout: u32,
        url: impl AsRef<str>,
    ) -> Result<Self> {
        let url = Url::parse(url.as_ref())?.join("v1")?;
        let session = Uuid::new_v4();
        let json = FalreSolverrCommand::CreateSession(session).to_json();

        client
            .post(url.as_str())
            .json(&json)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to create a FlareSolverr session {:?} at {:?}",
                    session,
                    url.as_str()
                )
            })?;

        Ok(Self {
            client,
            timeout,
            session,
            url,
        })
    }

    /// Check the status of the given URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to check.
    ///
    /// # Returns
    ///
    /// Returns a [`UrlStatus`] indicating the result of the check.
    pub(crate) async fn check(&self, url: impl Into<String>) -> UrlStatus {
        let url = url.into();
        let json = FalreSolverrCommand::Get {
            url: url.clone(),
            timeout: self.timeout,
            session: self.session,
        }
        .to_json();

        let status = self
            .client
            .post(self.url.as_str())
            .timeout(Duration::from_secs(self.timeout.into()))
            .header(CONTENT_TYPE, "application/json")
            .json(&json)
            .send()
            .await;

        match status {
            Ok(response) => match response.json::<FlareSolverrResponse>().await {
                Ok(response) => UrlStatus::from(response),
                Err(_) => UrlStatus { url, status: None },
            },
            Err(_) => UrlStatus { url, status: None },
        }
    }

    /// Close the FlareSolverr session associated with this client.
    ///
    /// # Errors
    ///
    /// Returns an error if the session could not be destroyed or if the FlareSolverr service is unreachable.
    pub(crate) async fn close(self) -> Result<()> {
        let json = FalreSolverrCommand::DestroySession(self.session).to_json();
        self.client
            .post(self.url.as_str())
            .json(&json)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to destroy FlareSolverr session {:?} at {:?}",
                    self.session, self.url
                )
            })?;
        Ok(())
    }
}
