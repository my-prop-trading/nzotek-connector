//! Client and webhook types for the Nzotek payment gateway.
//!
//! - [`api`]: merchant REST API (hosted-page deposits, order status), authenticated by the API key.
//! - [`models`]: request/response bodies and the order status shared by the API and webhooks.
//! - [`webhook`]: status callback body and its `x-pay-signature` check against the webhook key.
//!
//! Amounts are whole units of the order currency; INR rejects fractional amounts.

pub mod api;
pub mod models;
pub mod webhook;

pub use api::{NzotekApi, NzotekError, API_URL};
pub use models::*;
pub use webhook::{verify_signature, DepositWebhook, SIGNATURE_HEADER};
