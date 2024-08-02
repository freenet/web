use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SignCertificateRequest {
    pub blinded_message: String,
    pub amount_dollars: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignCertificateResponse {
    pub blind_signature: String,
}
