use crate::models::{CreateDepositRequest, CreateDepositResponse, OrderStatusResponse};
use flurl::body::HttpRequestBody;
use flurl::FlUrl;
use serde::de::DeserializeOwned;
use std::fmt;
use std::time::Duration;

/// Nzotek API host; test and live merchants differ by API key only.
pub const API_URL: &str = "https://api.nzotek.com/api";

const JSON_CONTENT_TYPE: &str = "application/json";

// Error pages carry full stack traces.
const MAX_RESPONSE_BODY_BYTES: usize = 1024 * 1024;

/// Failure of an Nzotek API call.
#[derive(Debug)]
pub enum NzotekError {
    /// 401: wrong or missing API key.
    Unauthorized,
    /// 402: the customer is blocked by Nzotek risk management.
    UserBlocked(String),
    /// 403: the caller's IP is not whitelisted by Nzotek.
    Forbidden(String),
    /// 422: the request failed validation; the body names the field.
    Validation(String),
    /// Any other non-success status.
    Http { status: u16, body: String },
    /// The request did not complete or the response body could not be read.
    Transport(String),
    /// A success body that does not match the expected model.
    Deserialize { body: String, error: String },
}

impl fmt::Display for NzotekError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "Nzotek 401: unauthorized"),
            Self::UserBlocked(body) => write!(f, "Nzotek 402: user blocked: {body}"),
            Self::Forbidden(body) => write!(f, "Nzotek 403: {body}"),
            Self::Validation(body) => write!(f, "Nzotek 422: {body}"),
            Self::Http { status, body } => write!(f, "Nzotek {status}: {body}"),
            Self::Transport(err) => write!(f, "Nzotek request failed: {err}"),
            Self::Deserialize { body, error } => {
                write!(f, "Nzotek response not parsed: {error}; body: {body}")
            }
        }
    }
}

impl std::error::Error for NzotekError {}

/// Client of the Nzotek merchant API.
pub struct NzotekApi {
    base_url: String,
    api_key: String,
    timeout: Duration,
}

impl NzotekApi {
    pub fn new(api_key: impl Into<String>, timeout: Duration) -> Self {
        Self::with_url(API_URL, api_key, timeout)
    }

    pub fn with_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout,
        }
    }

    /// Creates a deposit paid on the hosted page; returns the page URL and the Nzotek order id.
    pub async fn create_deposit(
        &self,
        request: &CreateDepositRequest,
    ) -> Result<CreateDepositResponse, NzotekError> {
        let body =
            serde_json::to_vec(request).map_err(|err| NzotekError::Transport(err.to_string()))?;
        let fl_url = self
            .fl_url()
            .append_path_segment("deposit")
            .append_path_segment("new")
            .with_header("Content-Type", JSON_CONTENT_TYPE);

        let response = fl_url
            .post(HttpRequestBody::from_raw_data(body, None))
            .await
            .map_err(|err| NzotekError::Transport(format!("{err:?}")))?;

        read_response(response).await
    }

    /// Fetches the current status of an order by our order id; returns its status and actual amount.
    /// Status checks are rate-limited by Nzotek: webhooks stay the primary status source.
    pub async fn get_order_status(
        &self,
        merchant_order_id: &str,
    ) -> Result<OrderStatusResponse, NzotekError> {
        let response = self
            .fl_url()
            .append_path_segment("merchantorder")
            .append_path_segment("status")
            .append_path_segment(merchant_order_id)
            .get()
            .await
            .map_err(|err| NzotekError::Transport(format!("{err:?}")))?;

        read_response(response).await
    }

    fn fl_url(&self) -> FlUrl {
        FlUrl::new(self.base_url.as_str())
            .set_timeout(self.timeout)
            .set_response_body_timeout(self.timeout)
            .with_header("Authorization", self.api_key.as_str())
            .with_header("Accept", JSON_CONTENT_TYPE)
    }
}

async fn read_response<T: DeserializeOwned>(
    mut response: flurl::FlUrlResponse,
) -> Result<T, NzotekError> {
    let status = response.get_status_code();
    let body = response
        .get_body()
        .map_err(|err| NzotekError::Transport(format!("{err:?}")))?
        .into_vec(MAX_RESPONSE_BODY_BYTES)
        .await
        .map_err(|err| NzotekError::Transport(format!("{err:?}")))?;
    let body = String::from_utf8_lossy(&body).into_owned();

    match status {
        200..=299 => serde_json::from_str(&body).map_err(|err| NzotekError::Deserialize {
            body,
            error: err.to_string(),
        }),
        401 => Err(NzotekError::Unauthorized),
        402 => Err(NzotekError::UserBlocked(body)),
        403 => Err(NzotekError::Forbidden(body)),
        422 => Err(NzotekError::Validation(body)),
        _ => Err(NzotekError::Http { status, body }),
    }
}
