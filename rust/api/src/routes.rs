use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use ghostkey_lib::armorable::Armorable;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use stripe::{Client, Currency, PaymentIntent, PaymentIntentId};

use crate::delegates::get_notary;
use crate::handle_sign_cert::{
    sign_certificate, CertificateError, SignCertificateRequest, SignCertificateResponse,
};
use crate::invite;
use crate::rate_limit::{
    AggregateBucket, RateLimiter, DEFAULT_TOR_INVITES_PER_HOUR, MAX_INVITES_PER_WINDOW,
    TOR_WINDOW_MINUTES,
};
use crate::tor::TorExitList;

/// Shared application state for invite generation
#[derive(Clone)]
pub struct InviteState {
    pub rate_limiter: Arc<RateLimiter>,
    /// Shared ceiling across the whole Tor exit set (see `AggregateBucket`).
    pub tor_bucket: Arc<AggregateBucket>,
    /// Membership test for "is this IP a Tor exit". Empty => nothing is Tor.
    pub tor_exits: Arc<TorExitList>,
    pub room_owner_vk: VerifyingKey,
    pub inviter_signing_key: SigningKey,
    pub room_name: String,
}

impl InviteState {
    pub fn new(
        rate_limit_file: PathBuf,
        tor_exit_cache: Option<PathBuf>,
        tor_invites_per_hour: Option<usize>,
        room_owner_vk: VerifyingKey,
        inviter_signing_key: SigningKey,
        room_name: String,
    ) -> Self {
        Self {
            rate_limiter: Arc::new(RateLimiter::new(rate_limit_file, 24)),
            tor_bucket: Arc::new(AggregateBucket::new(
                tor_invites_per_hour.unwrap_or(DEFAULT_TOR_INVITES_PER_HOUR),
                TOR_WINDOW_MINUTES,
            )),
            tor_exits: Arc::new(TorExitList::new(tor_exit_cache)),
            room_owner_vk,
            inviter_signing_key,
            room_name,
        }
    }
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
    status: u16,
}

#[derive(Serialize, Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize, Debug)]
pub struct DonationRequest {
    pub amount: i64,
}

/// HTTP response for donation create / update.
///
/// During the 0.2.0 rename transition the notary certificate is emitted in
/// BOTH `delegate_certificate_base64` (legacy) and `notary_certificate_base64`
/// (canonical) fields with identical values. See `SignCertificateResponse`
/// for the full rationale and freenet/web#24 for tracking.
#[derive(Serialize)]
pub struct DonationResponse {
    pub client_secret: String,
    pub payment_intent_id: String,
    pub delegate_certificate_base64: String,
    pub notary_certificate_base64: String,
}

async fn index() -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Hello, world!"
    }))
}

async fn get_message() -> impl IntoResponse {
    Json(Message {
        content: String::from("Welcome to the Freenet API! This message confirms that the API is functioning correctly."),
    })
}

async fn sign_certificate_route(
    Json(request): Json<SignCertificateRequest>,
) -> Result<Json<SignCertificateResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Received sign-certificate request: {:?}", request);
    match sign_certificate(request).await {
        Ok(response) => {
            info!("Certificate signed successfully");
            Ok(Json(response))
        }
        Err(e) => {
            error!("Error signing certificate: {:?}", e);
            match e {
                CertificateError::PaymentNotSuccessful => {
                    Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                        error: "Payment not successful. Please check your payment details and try again.".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    })))
                },
                CertificateError::CertificateAlreadySigned => {
                    Err((StatusCode::CONFLICT, Json(ErrorResponse {
                        error: "Certificate has already been signed for this payment.".to_string(),
                        status: StatusCode::CONFLICT.as_u16(),
                    })))
                },
                CertificateError::KeyError(msg) => {
                    Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                        error: format!("Key error: {}", msg),
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    })))
                },
                _ => {
                    Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                        error: "An unexpected error occurred. Please try again later.".to_string(),
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    })))
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum DonationError {
    InvalidCurrency,
    StripeError(stripe::StripeError),
    EnvError(std::env::VarError),
    OtherError(String),
}

impl IntoResponse for DonationError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            DonationError::InvalidCurrency => (StatusCode::BAD_REQUEST, "Invalid currency"),
            DonationError::StripeError(e) => {
                error!("Stripe error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Stripe error occurred")
            }
            DonationError::EnvError(e) => {
                error!("Environment variable error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Environment variable error",
                )
            }
            DonationError::OtherError(e) => {
                error!("Other error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An unexpected error occurred",
                )
            }
        };

        let body = Json(ErrorResponse {
            error: error_message.to_string(),
            status: status.as_u16(),
        });

        (status, body).into_response()
    }
}

async fn create_donation(
    Json(request): Json<DonationRequest>,
) -> Result<Json<DonationResponse>, DonationError> {
    info!("Received create-donation request: {:?}", request);

    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let currency = Currency::USD;

    let mut metadata = HashMap::new();
    metadata.insert("donation_type".to_string(), "freenet".to_string());

    let params = stripe::CreatePaymentIntent {
        amount: request.amount,
        currency,
        automatic_payment_methods: None,
        metadata: Some(metadata),
        description: Some("Freenet Donation"),
        statement_descriptor: Some("Freenet Donation"),
        statement_descriptor_suffix: Some("Thank You"),
        payment_method_types: Some(vec!["card".to_string()]),
        capture_method: None,
        confirm: None,
        setup_future_usage: None,
        confirmation_method: None,
        customer: None,
        error_on_requires_action: None,
        mandate: None,
        mandate_data: None,
        off_session: None,
        on_behalf_of: None,
        payment_method: None,
        payment_method_data: None,
        payment_method_options: Some(stripe::CreatePaymentIntentPaymentMethodOptions {
            card: Some(stripe::CreatePaymentIntentPaymentMethodOptionsCard {
                request_three_d_secure: Some(stripe::CreatePaymentIntentPaymentMethodOptionsCardRequestThreeDSecure::Automatic),
                ..Default::default()
            }),
            ..Default::default()
        }),
        receipt_email: None,
        return_url: None,
        shipping: None,
        transfer_data: None,
        transfer_group: None,
        application_fee_amount: None,
        use_stripe_sdk: None,
        expand: &[],
        payment_method_configuration: None,
        radar_options: None,
    };

    let intent = PaymentIntent::create(&client, params)
        .await
        .map_err(DonationError::StripeError)?;

    info!("Payment intent created successfully");

    let amount_dollars = request.amount / 100;

    let (notary_certificate, _) = get_notary(amount_dollars as u64).map_err(|e| {
        error!("Error getting notary: {:?}", e);
        DonationError::OtherError("Error getting notary".to_string())
    })?;

    let cert_base64 = notary_certificate.to_base64().unwrap();

    match intent.client_secret {
        Some(secret) => Ok(Json(DonationResponse {
            client_secret: secret,
            payment_intent_id: intent.id.to_string(),
            delegate_certificate_base64: cert_base64.clone(),
            notary_certificate_base64: cert_base64,
        })),
        None => {
            error!("Client secret is missing from the PaymentIntent");
            Err(DonationError::OtherError(
                "Client secret is missing".to_string(),
            ))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct UpdateDonationRequest {
    pub payment_intent_id: String,
    pub amount: i64,
}

async fn update_donation(
    Json(request): Json<UpdateDonationRequest>,
) -> Result<Json<DonationResponse>, DonationError> {
    info!("Received update-donation request: {:?}", request);

    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let payment_intent_id = PaymentIntentId::from_str(&request.payment_intent_id)
        .map_err(|_| DonationError::InvalidCurrency)?;
    let params = stripe::UpdatePaymentIntent {
        amount: Some(request.amount),
        ..Default::default()
    };

    let updated_intent = stripe::PaymentIntent::update(&client, &payment_intent_id, params)
        .await
        .map_err(DonationError::StripeError)?;

    info!("Payment intent updated successfully");

    let amount_dollars = request.amount / 100;

    let (notary_certificate, _) = get_notary(amount_dollars as u64).map_err(|e| {
        error!("Error getting notary: {:?}", e);
        DonationError::OtherError("Error getting notary".to_string())
    })?;

    let cert_base64 = notary_certificate.to_base64().unwrap();

    Ok(Json(DonationResponse {
        client_secret: updated_intent.client_secret.unwrap_or_default(),
        payment_intent_id: updated_intent.id.to_string(),
        delegate_certificate_base64: cert_base64.clone(),
        notary_certificate_base64: cert_base64,
    }))
}

async fn check_payment_status_route(
    Path(payment_intent_id): Path<String>,
) -> Result<StatusCode, DonationError> {
    info!(
        "Received check-payment-status request for PaymentIntent ID: {}",
        payment_intent_id
    );

    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let payment_intent_id = PaymentIntentId::from_str(&payment_intent_id)
        .map_err(|_| DonationError::InvalidCurrency)?;

    let intent = stripe::PaymentIntent::retrieve(&client, &payment_intent_id, &[])
        .await
        .map_err(DonationError::StripeError)?;

    if intent.status == stripe::PaymentIntentStatus::Succeeded {
        info!("Payment intent succeeded");
        Ok(StatusCode::OK)
    } else {
        error!("Payment intent not successful: {:?}", intent.status);
        Err(DonationError::OtherError(
            "Payment not successful".to_string(),
        ))
    }
}

// ============================================================================
// River Room Invite Endpoint
// ============================================================================

#[derive(Serialize)]
pub struct CreateInviteResponse {
    pub invite_code: String,
    pub room_name: String,
}

#[derive(Serialize)]
pub struct InviteErrorResponse {
    pub error: String,
    pub retry_after_seconds: Option<i64>,
}

/// Extract the client IP used to key the invite rate limiter.
///
/// We deliberately key on the TCP connection's peer address (`addr.ip()`) and
/// do NOT trust `X-Forwarded-For` / `X-Real-IP`. gkapi is directly
/// internet-facing (it binds :80/:443 and terminates its own TLS; there is no
/// reverse proxy in front), so those headers are fully client-controlled and
/// trusting them would let a spammer bypass the limit by rotating a spoofed
/// header. `addr.ip()` returns the bare `IpAddr` (no port), so both IPv4 and
/// IPv6 peers key correctly.
///
/// If a trusted reverse proxy / Cloudflare is ever placed in front of gkapi,
/// revisit HERE to trust `X-Forwarded-For` ONLY when the peer is that proxy's
/// IP. Do not re-add blanket `X-Forwarded-For` trust.
fn get_client_ip(addr: SocketAddr) -> IpAddr {
    addr.ip()
}

async fn create_room_invite(
    State(state): State<InviteState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<CreateInviteResponse>, (StatusCode, Json<InviteErrorResponse>)> {
    let client_ip = get_client_ip(addr);
    let via_tor = state.tor_exits.is_exit(&client_ip);
    info!(
        "Received create-invite request from IP: {} (tor_exit={})",
        client_ip, via_tor
    );

    // Tor exits share ONE hourly ceiling, because per-IP limiting cannot bound
    // an actor who rotates exit nodes (2026-07-25: 152 invites via 113 exits,
    // none near the per-IP limit).
    //
    // Ordering matters. This capacity check runs BEFORE `check_and_record` so a
    // request refused here does not also burn the requester's per-IP allowance
    // -- otherwise a burst would silently consume the quota of the ordinary Tor
    // users it is sharing the bucket with. The matching `record()` happens only
    // AFTER the per-IP check passes, so a request rejected for per-IP reasons
    // never consumes shared Tor capacity either.
    if via_tor && !state.tor_bucket.has_capacity() {
        let retry_after = state.tor_bucket.retry_after_seconds();
        // warn!, not info!: sustained exhaustion means legitimate Tor users are
        // being refused and an operator should see it.
        warn!(
            "Tor exit {} refused: shared Tor ceiling reached ({}/{}), retry_after: {:?}",
            client_ip,
            state.tor_bucket.current(),
            state.tor_bucket.limit(),
            retry_after
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(InviteErrorResponse {
                // Deliberately does NOT tell a privacy-tool user to stop using
                // it, and does NOT state the exact ceiling (which would tell an
                // attacker precisely what budget to drain).
                error: "Too many invite requests right now. Please try again shortly.".to_string(),
                retry_after_seconds: retry_after,
            }),
        ));
    }

    // Check rate limit
    match state.rate_limiter.check_and_record(client_ip) {
        Ok(true) => {
            // Request allowed, generate invite
        }
        Ok(false) => {
            // Rate limited
            let retry_after = state.rate_limiter.get_retry_after(client_ip).ok().flatten();
            info!(
                "Rate limited IP: {}, retry_after: {:?}",
                client_ip, retry_after
            );
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(InviteErrorResponse {
                    error: format!(
                        "Rate limited. You can request up to {MAX_INVITES_PER_WINDOW} invites per 24 hours."
                    ),
                    retry_after_seconds: retry_after,
                }),
            ));
        }
        Err(e) => {
            error!("Rate limiter error: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InviteErrorResponse {
                    error: "Internal server error. Please try again later.".to_string(),
                    retry_after_seconds: None,
                }),
            ));
        }
    }

    // The per-IP check passed, so charge the shared Tor ceiling. `try_acquire`
    // (not the earlier `has_capacity`) is the admission AUTHORITY: it prunes,
    // checks and records under one lock, so the ceiling holds however many
    // requests race, and stays correct if an `.await` is ever introduced
    // between the pre-check and here.
    //
    // Placed after the per-IP check so a per-IP rejection never consumes shared
    // capacity that ordinary Tor users depend on.
    if via_tor && !state.tor_bucket.try_acquire() {
        // Lost the race against a concurrent request since the pre-check.
        let retry_after = state.tor_bucket.retry_after_seconds();
        warn!(
            "Tor exit {} refused at acquire: shared ceiling reached ({}/{})",
            client_ip,
            state.tor_bucket.current(),
            state.tor_bucket.limit()
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(InviteErrorResponse {
                error: "Too many invite requests right now. Please try again shortly.".to_string(),
                retry_after_seconds: retry_after,
            }),
        ));
    }

    // Generate invite
    match invite::create_invitation(&state.room_owner_vk, &state.inviter_signing_key) {
        Ok(invite_code) => {
            info!("Generated invite for IP: {}", client_ip);
            Ok(Json(CreateInviteResponse {
                invite_code,
                room_name: state.room_name.clone(),
            }))
        }
        Err(e) => {
            error!("Failed to generate invite: {:?}", e);
            // No invite was issued, so give the Tor slot back. Otherwise a
            // burst of generation failures would lock out every Tor user for a
            // full window on top of the outage itself.
            if via_tor {
                state.tor_bucket.release();
            }
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InviteErrorResponse {
                    error: "Failed to generate invite. Please try again later.".to_string(),
                    retry_after_seconds: None,
                }),
            ))
        }
    }
}

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/message", get(get_message))
        .route("/sign-certificate", post(sign_certificate_route))
        .route("/create-donation", post(create_donation))
        .route("/update-donation", post(update_donation))
        .route(
            "/check-payment-status/:payment_intent_id",
            get(check_payment_status_route),
        )
}

/// Get routes that require invite state (for River room invites)
pub fn get_invite_routes(state: InviteState) -> Router {
    Router::new()
        .route("/create-invite", post(create_room_invite))
        .with_state(state)
}

#[cfg(test)]
mod invite_handler_tests {
    //! End-to-end tests for the invite handler's rate-limit wiring.
    //!
    //! Every unit below this is correct in isolation; the VALUE of the change
    //! lives in how they compose in `create_room_invite`. Three one-line
    //! regressions -- dropping `try_acquire`, inverting `via_tor`, or moving the
    //! acquire above the per-IP check -- each leave the unit tests fully green
    //! and the limiter fully defeated. These tests are what fail instead.

    use super::*;
    use crate::rate_limit::AggregateBucket;
    use crate::tor::TorExitList;
    use std::net::SocketAddr;
    use tempfile::TempDir;

    /// Build state with an injected exit list and a small ceiling.
    fn state_with(dir: &TempDir, exits: &[String], ceiling: usize) -> InviteState {
        let cache = dir.path().join("exits.txt");
        std::fs::write(&cache, exits.join("\n")).unwrap();
        let mut seed = [0u8; 32];
        seed[0] = 7;
        let signing_key = SigningKey::from_bytes(&seed);
        let owner = signing_key.verifying_key();
        InviteState {
            rate_limiter: Arc::new(RateLimiter::new(dir.path().join("rl.json"), 24)),
            tor_bucket: Arc::new(AggregateBucket::new(ceiling, 60)),
            tor_exits: Arc::new(TorExitList::new(Some(cache))),
            room_owner_vk: owner,
            inviter_signing_key: signing_key,
            room_name: "Test Room".to_string(),
        }
    }

    fn addr(ip: &str) -> SocketAddr {
        SocketAddr::new(ip.parse::<IpAddr>().unwrap(), 12345)
    }

    async fn request(state: &InviteState, ip: &str) -> StatusCode {
        match create_room_invite(State(state.clone()), ConnectInfo(addr(ip))).await {
            Ok(_) => StatusCode::OK,
            Err((code, _)) => code,
        }
    }

    /// THE regression test for the incident this change exists to fix: an actor
    /// rotating many distinct Tor exits must be capped in TOTAL, not per-IP.
    ///
    /// Fails if `try_acquire` is dropped, or if `via_tor` is inverted.
    #[tokio::test]
    async fn tor_exits_share_one_ceiling_across_rotating_ips() {
        const CEILING: usize = 5;
        let dir = tempfile::tempdir().unwrap();
        // 40 distinct exits, each used once -- exactly the burst's shape.
        let exits: Vec<String> = (1..=40).map(|i| format!("185.220.101.{i}")).collect();
        let state = state_with(&dir, &exits, CEILING);

        let mut ok = 0;
        let mut refused = 0;
        for i in 1..=40 {
            match request(&state, &format!("185.220.101.{i}")).await {
                StatusCode::OK => ok += 1,
                StatusCode::TOO_MANY_REQUESTS => refused += 1,
                other => panic!("unexpected status {other}"),
            }
        }
        assert_eq!(
            ok, CEILING,
            "rotation across 40 exits must yield exactly {CEILING} invites, got {ok}"
        );
        assert_eq!(refused, 40 - CEILING);
    }

    /// A non-Tor IP must be unaffected by a saturated Tor ceiling.
    ///
    /// Fails if `via_tor` is inverted or ignored.
    #[tokio::test]
    async fn non_tor_ip_unaffected_by_full_tor_bucket() {
        let dir = tempfile::tempdir().unwrap();
        let exits: Vec<String> = (1..=10).map(|i| format!("185.220.101.{i}")).collect();
        let state = state_with(&dir, &exits, 2);

        // Saturate via Tor.
        assert_eq!(request(&state, "185.220.101.1").await, StatusCode::OK);
        assert_eq!(request(&state, "185.220.101.2").await, StatusCode::OK);
        assert_eq!(
            request(&state, "185.220.101.3").await,
            StatusCode::TOO_MANY_REQUESTS
        );

        let before = state.tor_bucket.current();
        assert_eq!(
            request(&state, "203.0.113.9").await,
            StatusCode::OK,
            "a non-Tor IP must still be served"
        );
        assert_eq!(
            state.tor_bucket.current(),
            before,
            "a non-Tor request must not consume shared Tor capacity"
        );
    }

    /// Pins the second half of the ordering rationale: a request refused by the
    /// PER-IP limiter must not consume shared Tor capacity.
    ///
    /// Fails if `try_acquire` is moved above `check_and_record`.
    #[tokio::test]
    async fn per_ip_rejection_does_not_consume_tor_capacity() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &["185.220.101.1".to_string()], 100);

        // Exhaust this single exit's per-IP allowance.
        for _ in 0..MAX_INVITES_PER_WINDOW {
            assert_eq!(request(&state, "185.220.101.1").await, StatusCode::OK);
        }
        let consumed = state.tor_bucket.current();
        assert_eq!(consumed, MAX_INVITES_PER_WINDOW);

        // Further requests are per-IP rejections; they must NOT take Tor slots.
        for _ in 0..5 {
            assert_eq!(
                request(&state, "185.220.101.1").await,
                StatusCode::TOO_MANY_REQUESTS
            );
        }
        assert_eq!(
            state.tor_bucket.current(),
            consumed,
            "per-IP rejections must not consume shared Tor capacity"
        );
    }

    /// Pins the first half of the ordering rationale: a request refused by the
    /// TOR ceiling must not burn the requester's per-IP allowance, so an
    /// ordinary Tor user still has their full quota once capacity frees.
    #[tokio::test]
    async fn tor_ceiling_refusal_does_not_burn_per_ip_allowance() {
        let dir = tempfile::tempdir().unwrap();
        let exits: Vec<String> = (1..=10).map(|i| format!("185.220.101.{i}")).collect();
        let state = state_with(&dir, &exits, 1);

        assert_eq!(request(&state, "185.220.101.1").await, StatusCode::OK);
        // Victim is refused purely by the ceiling.
        for _ in 0..3 {
            assert_eq!(
                request(&state, "185.220.101.5").await,
                StatusCode::TOO_MANY_REQUESTS
            );
        }
        // Free the ceiling; the victim must still have all 4 per-IP invites.
        state.tor_bucket.release();
        let mut served = 0;
        for _ in 0..MAX_INVITES_PER_WINDOW {
            if request(&state, "185.220.101.5").await == StatusCode::OK {
                served += 1;
                state.tor_bucket.release(); // keep ceiling free for this probe
            }
        }
        assert_eq!(
            served, MAX_INVITES_PER_WINDOW,
            "ceiling refusals must not have consumed the victim's per-IP quota"
        );
    }

    /// The fail-open claim that actually matters: with no exit list, the
    /// endpoint behaves exactly as it did before this feature existed.
    #[tokio::test]
    async fn empty_exit_list_is_a_handler_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &[], 1); // ceiling of 1, but nothing is Tor

        for _ in 0..MAX_INVITES_PER_WINDOW {
            assert_eq!(request(&state, "185.220.101.1").await, StatusCode::OK);
        }
        assert_eq!(
            request(&state, "185.220.101.1").await,
            StatusCode::TOO_MANY_REQUESTS,
            "per-IP limit still applies"
        );
        assert_eq!(
            state.tor_bucket.current(),
            0,
            "an empty exit list must never consume Tor capacity"
        );
    }

    /// The runtime off-switch must fully disable the ceiling.
    #[tokio::test]
    async fn zero_ceiling_disables_tor_metering() {
        let dir = tempfile::tempdir().unwrap();
        let exits: Vec<String> = (1..=20).map(|i| format!("185.220.101.{i}")).collect();
        let state = state_with(&dir, &exits, 0);

        for i in 1..=20 {
            assert_eq!(
                request(&state, &format!("185.220.101.{i}")).await,
                StatusCode::OK,
                "ceiling 0 must never refuse"
            );
        }
    }
}
