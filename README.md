# nzotek-connector

Rust client and webhook types for the [Nzotek](https://doc.nzotek.com/) payment gateway.

| Item | Purpose |
|------|---------|
| `NzotekApi::create_deposit` | `POST /deposit/new`: hosted-page deposit, returns `form_url` and the Nzotek order id |
| `NzotekApi::get_order_status` | `GET /merchantorder/status/{merchant_order_id}` |
| `DepositWebhook` | Status callback body |
| `verify_signature` | Checks the `x-pay-signature` header against the raw body and the webhook key |

| Setting | Where it comes from |
|---------|---------------------|
| API key | Nzotek merchant back office |
| Webhook key | Nzotek merchant back office |
| Caller IP | Whitelisted by Nzotek support |
