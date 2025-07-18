mod flaresolverr;

use crate::flaresolverr::FlareSolverrClient;
use anyhow::Result;
use async_stream::stream;
use bon::bon;
use futures::Stream;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Represents the status of a URL, including its HTTP status code if available.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UrlStatus {
    /// The URL as a string.
    pub url: String,
    /// The HTTP status code returned for the URL, if available.
    pub status: Option<u16>,
}

/// A utility for checking the availability and HTTP status of URLs, with optional
/// integration for FlareSolverr to bypass anti-bot challenges.
#[derive(Clone, Debug)]
pub struct LinkChecker {
    /// The HTTP client used for making requests.
    client: Client,
    /// An optional client for interacting with the FlareSolverr service.
    /// If `None`, FlareSolverr integration is disabled.
    flaresolverr: Option<FlareSolverrClient>,
}

impl Default for LinkChecker {
    /// Initializes a `LinkChecker` with a default `Client` configured
    /// with a 10-second timeout and no FlareSolverr integration.
    ///
    /// # Panics
    /// Panics if the underlying [Client::builder] panics.
    /// Use [Self::builder] if you want to handle the error.
    fn default() -> Self {
        Self {
            client: Client::builder().timeout(DEFAULT_TIMEOUT).build().unwrap(),
            flaresolverr: None,
        }
    }
}

/// Implements the core functionality for `LinkChecker`.
#[bon]
impl LinkChecker {
    /// Creates a new `LinkChecker` instance.
    #[builder]
    pub async fn new(
        /// The HTTP client to be used for all requests.
        client: Option<Client>,
        /// An optional URL for the FlareSolverr service.
        ///
        /// If provided, FlareSolverr integration is enabled using this URL,
        /// and a new session will be established. Otherwise, FlareSolverr
        /// will not be used for link checking.
        flaresolverr: Option<String>,
    ) -> Result<Self> {
        let client = client
            .map(Ok)
            .unwrap_or_else(|| Client::builder().timeout(DEFAULT_TIMEOUT).build())?;
        let flaresolverr = if let Some(url) = flaresolverr {
            Some(FlareSolverrClient::new(client.clone(), 60, url).await?)
        } else {
            None
        };
        Ok(Self {
            client,
            flaresolverr,
        })
    }

    /// An internal asynchronous helper function to perform a single URL check.
    async fn checker(
        url: String,
        client: Client,
        flaresolverr: Option<FlareSolverrClient>,
    ) -> UrlStatus {
        let result = &client.get(&url).send().await;

        match result {
            Ok(response) => {
                let code = response.status();
                // If a 403 Forbidden status is received, try with FlareSolverr if available
                if code == StatusCode::FORBIDDEN {
                    if let Some(solver) = flaresolverr {
                        solver.check(&url).await
                    } else {
                        // If no FlareSolverr, return the 403 status directly
                        UrlStatus {
                            url,
                            status: Some(code.as_u16()),
                        }
                    }
                } else {
                    // For any other status code, return it directly
                    UrlStatus {
                        url,
                        status: Some(code.as_u16()),
                    }
                }
            }
            // If the direct request fails (e.g., network error), return UrlStatus with no status code
            Err(_) => UrlStatus { url, status: None },
        }
    }

    /// Checks the status of a single URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to check.
    ///
    /// # Returns
    ///
    /// The status of the checked URL.
    pub async fn check(&self, url: impl Into<String>) -> UrlStatus {
        Self::checker(url.into(), self.client.clone(), self.flaresolverr.clone()).await
    }

    /// Checks the status of multiple URLs concurrently.
    ///
    /// # Arguments
    ///
    /// * `urls` - An iterator over items that can be converted into `String`.
    ///
    /// # Returns
    ///
    /// A vector containing the `UrlStatus` for each unique URL provided.
    pub async fn check_all<I, S>(&self, urls: I) -> impl Stream<Item = UrlStatus>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut tasks = tokio::task::JoinSet::new();
        // Spawn a new task for each URL check
        for url in urls {
            tasks.spawn(Self::checker(
                url.into(),
                self.client.clone(),
                self.flaresolverr.clone(),
            ));
        }
        // Wait for all tasks to complete and collect their results
        stream! {
            while let Some(task) = tasks.join_next().await {
            if let Ok(status) = task {
                yield status;
            }
        }
        }
    }

    /// Close the `LinkChecker` instance, specifically destroying the FlareSolverr
    /// session if one was active.
    ///
    /// Each instance of `LinkChecker` establishes a new session with the FlareSolverr service.
    /// Sessions must be explicitly destroyed using [`close`](Self::close) when no longer needed.
    /// Accumulating too many active sessions can degrade FlareSolverr performance.
    ///
    /// # Returns
    ///
    /// An [`anyhow::Result`] indicating success or an error if the FlareSolverr
    /// session could not be destroyed.
    pub async fn close(self) -> Result<()> {
        // If a FlareSolverr client exists, close its session
        if let Some(solverr) = self.flaresolverr {
            solverr.close().await?
        }
        Ok(())
    }
}
