use serde::{Deserialize, Deserializer, Serialize};

/// Deposit payment method enabled for the merchant account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepositMethod {
    Upi,
    BankTransfer,
}

/// Traffic type of a deposit: the customer's first purchase or a repeat one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Traffic {
    Ftd,
    Std,
}

/// Order status reported by the API and status webhooks.
///
/// `Cancelled` and `Expired` are not final: a deposit may still move to `Success` or `Failed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    New,
    Pending,
    Success,
    Failed,
    Cancelled,
    Expired,
    Refunded,
    Chargeback,
    #[serde(other)]
    Unknown,
}

/// Body of `POST /deposit/new`: a deposit paid on the Nzotek hosted page.
#[derive(Debug, Clone, Serialize)]
pub struct CreateDepositRequest {
    pub method: DepositMethod,
    /// Whole units of `currency`.
    pub amount: u64,
    /// ISO 4217 code; selects the market.
    pub currency: String,
    pub traffic: Traffic,
    /// Unique per deposit: a repeated id is rejected.
    pub merchant_order_id: String,
    pub merchant_customer_id: String,
    pub status_url: String,
    pub redirect_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_email: Option<String>,
    /// `+` and the country code; India also requires 10 digits starting with 6-9.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_name: Option<String>,
}

/// Created deposit: the hosted page to show the customer and the Nzotek order id.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDepositResponse {
    /// Opened by redirect or in an iframe.
    pub form_url: String,
    /// Nzotek order id; not to be shown to the customer.
    pub order_id: String,
    pub status: OrderStatus,
}

/// Current state of an order as returned by the status endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct OrderStatusResponse {
    pub order_id: String,
    pub merchant_order_id: String,
    pub status: OrderStatus,
    #[serde(default)]
    pub reason: Option<String>,
    /// Actual amount, which may differ from the requested one.
    #[serde(deserialize_with = "amount_as_string")]
    pub amount: String,
    #[serde(default)]
    pub utr: Option<String>,
}

/// Reads an amount sent either as a JSON string or as a number.
pub(crate) fn amount_as_string<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Amount {
        Text(String),
        Number(serde_json::Number),
    }

    Ok(match Amount::deserialize(deserializer)? {
        Amount::Text(text) => text,
        Amount::Number(number) => number.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_request_serializes_whole_amount_and_skips_missing_contacts() {
        let request = CreateDepositRequest {
            method: DepositMethod::Upi,
            amount: 2000,
            currency: "INR".to_string(),
            traffic: Traffic::Ftd,
            merchant_order_id: "order-1".to_string(),
            merchant_customer_id: "client-1".to_string(),
            status_url: "https://example.com/status".to_string(),
            redirect_url: "https://example.com/return".to_string(),
            customer_email: Some("a@b.c".to_string()),
            customer_phone: None,
            customer_name: None,
        };

        let json: serde_json::Value = serde_json::to_value(&request).unwrap();

        assert_eq!(json["method"], "upi");
        assert_eq!(json["amount"], 2000);
        assert_eq!(json["traffic"], "ftd");
        assert_eq!(json["customer_email"], "a@b.c");
        assert!(json.get("customer_phone").is_none());
        assert!(json.get("customer_name").is_none());
    }

    #[test]
    fn deposit_response_parses_doc_example() {
        let body = r#"{"form_url":"https://linestrack.com/wire?hash=f16f937b","order_id":"e7d414e6-81b6-4dd9-a785-d19cb315c118","status":"new"}"#;

        let response: CreateDepositResponse = serde_json::from_str(body).unwrap();

        assert_eq!(response.order_id, "e7d414e6-81b6-4dd9-a785-d19cb315c118");
        assert_eq!(response.status, OrderStatus::New);
    }

    #[test]
    fn status_response_parses_expired_order_with_null_utr() {
        let body = r#"{"order_id":"75df4f3a","merchant_order_id":"8a42590b","status":"expired","reason":"new -> expired: Form expired","amount":"2000.00","utr":null}"#;

        let response: OrderStatusResponse = serde_json::from_str(body).unwrap();

        assert_eq!(response.status, OrderStatus::Expired);
        assert_eq!(response.amount, "2000.00");
        assert_eq!(response.utr, None);
    }

    #[test]
    fn unknown_status_does_not_fail_parsing() {
        let status: OrderStatus = serde_json::from_str(r#""on_review""#).unwrap();

        assert_eq!(status, OrderStatus::Unknown);
    }
}
