#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_generate_stripe_secret_key() {
        let key = generate_stripe_secret_key();
        env::set_var("STRIPE_SECRET_KEY", key);

        let request = SignCertificateRequest {
            payment_intent_id: "test_payment_intent".to_string(),
            blinded_public_key: "test_blinded_public_key".to_string(),
        };

        let result = sign_certificate(request);
        assert!(result.is_ok());
    }
}
