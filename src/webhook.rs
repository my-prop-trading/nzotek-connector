use crate::models::{amount_as_string, OrderStatus};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

/// Header carrying the HMAC-SHA256 of the raw webhook body, keyed by the webhook key.
pub const SIGNATURE_HEADER: &str = "x-pay-signature";

type HmacSha256 = Hmac<Sha256>;

/// Status callback of a deposit order, sent to the order's `status_url` on every status change.
#[derive(Debug, Clone, Deserialize)]
pub struct DepositWebhook {
    pub order_id: String,
    pub merchant_order_id: String,
    pub status: OrderStatus,
    #[serde(default)]
    pub reason: Option<String>,
    /// Actually paid amount, which may differ from the requested one; the original amount on `Chargeback`.
    #[serde(deserialize_with = "amount_as_string")]
    pub amount: String,
    #[serde(default)]
    pub merchant_customer_id: Option<String>,
    /// Bank transaction reference (India).
    #[serde(default)]
    pub utr: Option<String>,
    #[serde(default)]
    pub target_upi_id: Option<String>,
    /// Complaint reference, present on `Chargeback`.
    #[serde(default)]
    pub claim_id: Option<String>,
}

/// Checks the `x-pay-signature` of a webhook against its raw body; returns true if it matches.
/// The body must be the bytes as received, before any JSON parsing.
pub fn verify_signature(webhook_key: &str, body: &[u8], signature: &str) -> bool {
    let signature = signature.trim();
    let mut mac = HmacSha256::new_from_slice(webhook_key.as_bytes())
        .expect("HMAC accepts a key of any length");
    mac.update(body);

    // The encoding is not documented, so hex and base64 are both accepted.
    [hex::decode(signature).ok(), STANDARD.decode(signature).ok()]
        .into_iter()
        .flatten()
        .any(|expected| mac.clone().verify_slice(&expected).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "key";
    const BODY: &[u8] = b"The quick brown fox jumps over the lazy dog";
    const HEX_SIGNATURE: &str = "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8";
    const BASE64_SIGNATURE: &str = "97yD9DBThCSxMpjmqm+xQ+9NWaFJRhdZl0edvC0aPNg=";

    #[test]
    fn accepts_hex_signature_in_any_case() {
        assert!(verify_signature(KEY, BODY, HEX_SIGNATURE));
        assert!(verify_signature(KEY, BODY, &HEX_SIGNATURE.to_uppercase()));
    }

    #[test]
    fn accepts_base64_signature() {
        assert!(verify_signature(KEY, BODY, BASE64_SIGNATURE));
    }

    #[test]
    fn rejects_tampered_body_wrong_key_and_garbage() {
        assert!(!verify_signature(
            KEY,
            b"The quick brown fox jumps over the lazy cat",
            HEX_SIGNATURE
        ));
        assert!(!verify_signature("other", BODY, HEX_SIGNATURE));
        assert!(!verify_signature(KEY, BODY, ""));
        assert!(!verify_signature(KEY, BODY, "not a signature"));
    }

    #[test]
    fn parses_doc_chargeback_webhook() {
        let body = r#"{"order_id":"849655cf-ceba-49bc-951a-cbfbf8d63274","merchant_order_id":"O_XVajPtgGFk173LluM5oKb","status":"chargeback","reason":"chargeback","amount":"200.00","utr":"409876543210","target_upi_id":"deposit@test","merchant_customer_id":"aaaaaaa99-1290-3e9f-a08e-c03aedc68b35","claim_id":"cc-1"}"#;

        let webhook: DepositWebhook = serde_json::from_str(body).unwrap();

        assert_eq!(webhook.status, OrderStatus::Chargeback);
        assert_eq!(webhook.amount, "200.00");
        assert_eq!(webhook.claim_id.as_deref(), Some("cc-1"));
    }

    #[test]
    fn parses_pending_webhook_with_numeric_amount_and_extra_fields() {
        let body = r#"{"order_id":"a","merchant_order_id":"b","status":"pending","amount":200,"holder_name":"x"}"#;

        let webhook: DepositWebhook = serde_json::from_str(body).unwrap();

        assert_eq!(webhook.status, OrderStatus::Pending);
        assert_eq!(webhook.amount, "200");
        assert_eq!(webhook.utr, None);
    }
}
