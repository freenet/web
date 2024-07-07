#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    use rocket_api::stripe_handler::{sign_certificate, SignCertificateRequest};
    use rocket_api::fn_key_util::generate_stripe_secret_key;
    use std::env;

    #[tokio::test]
    async fn test_generate_stripe_secret_key() {
        let key = generate_stripe_secret_key();
        env::set_var("STRIPE_SECRET_KEY", key);

        let request = SignCertificateRequest {
            payment_intent_id: "test_payment_intent".to_string(),
            blinded_public_key: "test_blinded_public_key".to_string(),
        };

        let result = sign_certificate(request).await;
        assert!(result.is_ok());
    }
}
