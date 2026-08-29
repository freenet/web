use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
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
use crate::invite_pow::{PowChallenge, PowChallengeResponse, PowError, PowManager};
use crate::rate_limit::{
    AggregateBucket, RateLimiter, DEFAULT_GLOBAL_INVITES_PER_HOUR, GLOBAL_WINDOW_MINUTES,
    MAX_INVITES_PER_WINDOW,
};
use crate::tor::TorExitList;
use tower_http::cors::CorsLayer;

/// Shared application state for invite generation
#[derive(Clone)]
pub struct InviteState {
    pub rate_limiter: Arc<RateLimiter>,
    /// Emergency ceiling across all successful invitation issuance.
    pub global_bucket: Arc<AggregateBucket>,
    pub pow: Arc<PowManager>,
    /// Membership test for "is this IP a Tor exit". An empty list makes the
    /// invite endpoint fail closed until the first refresh succeeds.
    pub tor_exits: Arc<TorExitList>,
    pub room_owner_vk: VerifyingKey,
    pub inviter_signing_key: SigningKey,
    pub room_name: String,
}

impl InviteState {
    pub fn new(
        rate_limit_file: PathBuf,
        tor_exit_cache: Option<PathBuf>,
        global_invites_per_hour: Option<usize>,
        pow_base_difficulty: u8,
        room_owner_vk: VerifyingKey,
        inviter_signing_key: SigningKey,
        room_name: String,
    ) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(rate_limit_file, 24));
        let tor_exits = Arc::new(TorExitList::new(tor_exit_cache));
        let recent_ages = match rate_limiter.recent_events(GLOBAL_WINDOW_MINUTES) {
            // Seed only traffic the new policy would have admitted. Otherwise
            // a pre-deploy Tor wave consumes legitimate global headroom even
            // though every equivalent request is blocked after the restart.
            Ok(events) => events
                .into_iter()
                .filter_map(|(ip, age)| (!tor_exits.is_exit(&ip)).then_some(age))
                .collect(),
            Err(e) => {
                warn!("Could not seed global invite ceiling from persistent state: {e}");
                Vec::new()
            }
        };
        info!(
            "Seeding global invite ceiling with {} invitation(s) from the last hour",
            recent_ages.len()
        );
        Self {
            rate_limiter,
            global_bucket: Arc::new(AggregateBucket::new_seeded(
                global_invites_per_hour.unwrap_or(DEFAULT_GLOBAL_INVITES_PER_HOUR),
                GLOBAL_WINDOW_MINUTES,
                recent_ages,
            )),
            pow: Arc::new(PowManager::new(pow_base_difficulty)),
            tor_exits,
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

#[derive(Deserialize)]
struct CreateInviteRequest {
    #[serde(flatten)]
    challenge: PowChallenge,
    nonce: u64,
}

/// Extract the client IP used to key the invite rate limiter.
///
/// We deliberately key on the TCP connection's peer address (`addr.ip()`) and
/// trust `X-Forwarded-For` ONLY when the TCP peer is loopback, i.e. our own
/// caddy reverse proxy on the same host. For any other peer we key on
/// `addr.ip()`, because those headers are fully client-controlled and blanket
/// trust would let a spammer bypass the limit by rotating a spoofed header.
///
/// This became load-bearing on 2026-08-28 when gkapi moved from vega (where it
/// bound :80/:443 directly) to nova behind caddy. Every request then arrived
/// from 127.0.0.1, so ALL users collapsed into one rate-limit bucket and the
/// 5th invite in the window was refused for everyone. Do not re-add blanket
/// `X-Forwarded-For` trust, and do not remove the loopback guard.
fn get_client_ip(addr: SocketAddr, headers: &HeaderMap) -> IpAddr {
    // Only a loopback peer is our own reverse proxy. A direct internet client
    // can never present one, so header trust cannot be reached from outside.
    // `to_canonical()` first: a dual-stack listener presents a loopback peer as
    // the IPv4-mapped `::ffff:127.0.0.1`, for which `is_loopback()` is FALSE.
    // Without the unmapping this fails CLOSED into the very bug it fixes — the
    // header is ignored, every request keys on the proxy address, and all users
    // collapse into one rate-limit bucket. Today's socket is IPv4-only and caddy
    // targets 127.0.0.1 explicitly, so it does not bite; this keeps it from
    // biting if either ever changes.
    if !addr.ip().to_canonical().is_loopback() {
        return addr.ip();
    }
    // Caddy APPENDS the real peer to any client-supplied X-Forwarded-For, so the
    // RIGHTMOST entry is the one caddy added and the only one not attacker-set.
    // Taking the leftmost here would restore exactly the spoofing hole the
    // doc comment above warns about.
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .map(str::trim)
        .and_then(|v| v.parse::<IpAddr>().ok())
        .unwrap_or_else(|| addr.ip())
}

fn invite_error(
    status: StatusCode,
    message: impl Into<String>,
    retry_after_seconds: Option<i64>,
) -> (StatusCode, Json<InviteErrorResponse>) {
    (
        status,
        Json(InviteErrorResponse {
            error: message.into(),
            retry_after_seconds,
        }),
    )
}

/// Enforce the network-level admission policy before issuing a challenge or
/// accepting proof of work. Tor is intentionally blocked for this public room:
/// rotating exits defeated IP rate limiting during the July 2026 spam waves.
fn check_invite_network(
    state: &InviteState,
    client_ip: IpAddr,
) -> Result<(), (StatusCode, Json<InviteErrorResponse>)> {
    if state.tor_exits.is_empty() {
        error!(
            "Invite request from {} refused: Tor exit list is unavailable",
            client_ip
        );
        return Err(invite_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Invitations are temporarily unavailable. Please try again shortly.",
            Some(30),
        ));
    }
    if state.tor_exits.is_exit(&client_ip) {
        warn!("Invite request blocked from Tor exit: {}", client_ip);
        return Err(invite_error(
            StatusCode::FORBIDDEN,
            "Invitations are not available from this network.",
            None,
        ));
    }
    Ok(())
}

async fn get_invite_challenge(
    State(state): State<InviteState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<PowChallengeResponse>, (StatusCode, Json<InviteErrorResponse>)> {
    let client_ip = get_client_ip(addr, &headers);
    check_invite_network(&state, client_ip)?;

    if !state.global_bucket.has_capacity() {
        let retry_after = state.global_bucket.retry_after_seconds();
        warn!(
            "Invite challenge refused: global ceiling reached ({}/{}), IP: {}",
            state.global_bucket.current(),
            state.global_bucket.limit(),
            client_ip
        );
        return Err(invite_error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many invite requests right now. Please try again shortly.",
            retry_after,
        ));
    }

    match state.rate_limiter.get_retry_after(client_ip) {
        Ok(Some(retry_after)) => {
            return Err(invite_error(
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "Rate limited. You can request up to {MAX_INVITES_PER_WINDOW} invites per 24 hours."
                ),
                Some(retry_after),
            ));
        }
        Ok(None) => {}
        Err(e) => {
            error!("Rate limiter error while issuing challenge: {e:?}");
            return Err(invite_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error. Please try again later.",
                None,
            ));
        }
    }

    Ok(Json(state.pow.issue(state.global_bucket.current())))
}

async fn create_room_invite(
    State(state): State<InviteState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<CreateInviteRequest>,
) -> Result<Json<CreateInviteResponse>, (StatusCode, Json<InviteErrorResponse>)> {
    let client_ip = get_client_ip(addr, &headers);
    check_invite_network(&state, client_ip)?;
    info!("Received create-invite request from IP: {}", client_ip);

    let proof_id = match state
        .pow
        .verify_and_consume(&request.challenge, request.nonce)
    {
        Ok(id) => id,
        Err(e) => {
            let status = match e {
                PowError::Expired => StatusCode::GONE,
                PowError::Reused => StatusCode::CONFLICT,
                PowError::Lock => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::BAD_REQUEST,
            };
            warn!("Invalid invite proof from {}: {}", client_ip, e);
            return Err(invite_error(
                status,
                "The invite verification could not be completed. Please try again.",
                None,
            ));
        }
    };

    // The global bucket is the final safety valve. Acquire it atomically before
    // recording the per-IP allowance, and refund it on all downstream failures.
    if !state.global_bucket.try_acquire() {
        state.pow.release(&proof_id);
        let retry_after = state.global_bucket.retry_after_seconds();
        warn!(
            "Invite refused at acquire: global ceiling reached ({}/{}), IP: {}",
            state.global_bucket.current(),
            state.global_bucket.limit(),
            client_ip
        );
        return Err(invite_error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many invite requests right now. Please try again shortly.",
            retry_after,
        ));
    }

    match state.rate_limiter.check_and_record(client_ip) {
        Ok(true) => {}
        Ok(false) => {
            state.global_bucket.release();
            state.pow.release(&proof_id);
            let retry_after = state.rate_limiter.get_retry_after(client_ip).ok().flatten();
            info!(
                "Rate limited IP: {}, retry_after: {:?}",
                client_ip, retry_after
            );
            return Err(invite_error(
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "Rate limited. You can request up to {MAX_INVITES_PER_WINDOW} invites per 24 hours."
                ),
                retry_after,
            ));
        }
        Err(e) => {
            state.global_bucket.release();
            state.pow.release(&proof_id);
            error!("Rate limiter error: {:?}", e);
            return Err(invite_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error. Please try again later.",
                None,
            ));
        }
    }

    match invite::create_invitation(&state.room_owner_vk, &state.inviter_signing_key) {
        Ok(created) => {
            info!(
                "Generated invite for IP: {} member_id={}",
                client_ip, created.member_id
            );
            Ok(Json(CreateInviteResponse {
                invite_code: created.code,
                room_name: state.room_name.clone(),
            }))
        }
        Err(e) => {
            error!("Failed to generate invite: {:?}", e);
            state.global_bucket.release();
            state.pow.release(&proof_id);
            Err(invite_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate invite. Please try again later.",
                None,
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
        .layer(CorsLayer::permissive())
}

/// Get routes that require invite state (for River room invites)
pub fn get_invite_routes(state: InviteState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("https://freenet.org"),
            HeaderValue::from_static("https://www.freenet.org"),
            HeaderValue::from_static("http://localhost:1313"),
            HeaderValue::from_static("http://127.0.0.1:1313"),
        ])
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);
    Router::new()
        .route("/invite-challenge", get(get_invite_challenge))
        .route("/create-invite", post(create_room_invite))
        .with_state(state)
        .layer(cors)
}

#[cfg(test)]
mod invite_handler_tests {
    /// #4 client-IP keying behind the caddy reverse proxy (2026-08-28 regression).
    ///
    /// When gkapi moved from vega (direct :443) to nova (behind caddy), every
    /// request arrived from 127.0.0.1, so ALL users collapsed into ONE rate-limit
    /// bucket and the 5th invite in the window was refused for everyone. These
    /// cases pin both halves of the fix: the real client is recovered behind the
    /// proxy, AND a client-supplied header is still not trusted.
    #[test]
    fn client_ip_trusts_forwarded_for_only_from_loopback() {
        use std::net::{Ipv4Addr, SocketAddr};

        let loopback = SocketAddr::from((Ipv4Addr::LOCALHOST, 40000));
        let external = SocketAddr::from((Ipv4Addr::new(203, 0, 113, 7), 40000));

        let mut hdr = HeaderMap::new();
        hdr.insert("x-forwarded-for", "9.9.9.9, 198.51.100.4".parse().unwrap());

        // Behind the proxy: caddy APPENDS the true peer, so the RIGHTMOST entry
        // wins. Taking the leftmost would trust the attacker-supplied 9.9.9.9.
        assert_eq!(
            get_client_ip(loopback, &hdr),
            "198.51.100.4".parse::<IpAddr>().unwrap(),
            "rightmost XFF entry must win behind the proxy"
        );

        // Direct internet peer: header is fully client-controlled, ignore it.
        assert_eq!(
            get_client_ip(external, &hdr),
            external.ip(),
            "XFF must never be trusted from a non-loopback peer"
        );

        // A PRIVATE address is not loopback either. Without this case, widening
        // the guard to `is_loopback() || is_private()` passes — and nova has
        // other hosts on its networks.
        let private = SocketAddr::from((Ipv4Addr::new(10, 0, 0, 1), 40000));
        assert_eq!(
            get_client_ip(private, &hdr),
            private.ip(),
            "only LOOPBACK is the trusted proxy; private ranges are not"
        );

        // A dual-stack listener presents an EXTERNAL v4 client as IPv4-mapped
        // too, not only loopback. Widening the guard to "any IPv4-mapped
        // address" passes every other case in this test and is then blanket
        // X-Forwarded-For trust, because under that listener EVERY v4 client
        // arrives mapped.
        let mapped_external: SocketAddr = "[::ffff:203.0.113.7]:40000".parse().unwrap();
        assert_eq!(
            get_client_ip(mapped_external, &hdr),
            mapped_external.ip(),
            "an IPv4-MAPPED external peer is still external; XFF must be ignored"
        );

        // A dual-stack listener presents loopback as IPv4-mapped IPv6. If this
        // is not unmapped, the header is ignored and every user collapses into
        // one bucket — the original bug, reintroduced through a different
        // address representation.
        let mapped: SocketAddr = "[::ffff:127.0.0.1]:40000".parse().unwrap();
        assert!(
            !mapped.ip().is_loopback(),
            "precondition: raw is_loopback is false here"
        );
        assert_eq!(
            get_client_ip(mapped, &hdr),
            "198.51.100.4".parse::<IpAddr>().unwrap(),
            "IPv4-mapped loopback must be treated as loopback"
        );

        // If caddy's appended value is unparseable we must NOT fall back to an
        // earlier, attacker-supplied entry. Taking the rightmost PARSEABLE entry
        // instead of the rightmost entry would hand back 9.9.9.9 here.
        let mut trailing_junk = HeaderMap::new();
        trailing_junk.insert("x-forwarded-for", "9.9.9.9, not-an-ip".parse().unwrap());
        assert_eq!(
            get_client_ip(loopback, &trailing_junk),
            loopback.ip(),
            "unparseable rightmost entry must fall back to the peer, never to an earlier entry"
        );

        // Loopback with no/!unparseable header falls back to the peer.
        assert_eq!(get_client_ip(loopback, &HeaderMap::new()), loopback.ip());
        let mut junk = HeaderMap::new();
        junk.insert("x-forwarded-for", "not-an-ip".parse().unwrap());
        assert_eq!(get_client_ip(loopback, &junk), loopback.ip());
    }

    use super::*;
    use crate::invite_pow::{valid_proof, PowManager};
    use tempfile::TempDir;

    fn state_with(dir: &TempDir, exits: &[&str], ceiling: usize) -> InviteState {
        let cache = dir.path().join("exits.txt");
        std::fs::write(&cache, exits.join("\n")).unwrap();
        let mut seed = [0u8; 32];
        seed[0] = 7;
        let signing_key = SigningKey::from_bytes(&seed);
        InviteState {
            rate_limiter: Arc::new(RateLimiter::new(dir.path().join("rl.json"), 24)),
            global_bucket: Arc::new(AggregateBucket::new(ceiling, 60)),
            pow: Arc::new(PowManager::new(4)),
            tor_exits: Arc::new(TorExitList::new(Some(cache))),
            room_owner_vk: signing_key.verifying_key(),
            inviter_signing_key: signing_key,
            room_name: "Test Room".to_string(),
        }
    }

    fn addr(ip: &str) -> SocketAddr {
        SocketAddr::new(ip.parse().unwrap(), 12345)
    }

    fn solve(challenge: PowChallenge) -> CreateInviteRequest {
        let id: [u8; 16] = hex::decode(&challenge.challenge)
            .unwrap()
            .try_into()
            .unwrap();
        let nonce = (0..u64::MAX)
            .find(|nonce| valid_proof(&id, *nonce, challenge.difficulty))
            .unwrap();
        CreateInviteRequest { challenge, nonce }
    }

    async fn challenge(state: &InviteState, ip: &str) -> Result<PowChallenge, StatusCode> {
        get_invite_challenge(
            State(state.clone()),
            ConnectInfo(addr(ip)),
            HeaderMap::new(),
        )
        .await
        .map(|response| response.0.challenge)
        .map_err(|(status, _)| status)
    }

    async fn request_with(
        state: &InviteState,
        ip: &str,
        request: CreateInviteRequest,
    ) -> StatusCode {
        match create_room_invite(
            State(state.clone()),
            ConnectInfo(addr(ip)),
            HeaderMap::new(),
            Json(request),
        )
        .await
        {
            Ok(_) => StatusCode::OK,
            Err((code, _)) => code,
        }
    }

    async fn request(state: &InviteState, ip: &str) -> StatusCode {
        let proof = solve(challenge(state, ip).await.unwrap());
        request_with(state, ip, proof).await
    }

    /// Same as `challenge`/`request_with`, but arriving the way production
    /// traffic actually does: from the loopback proxy, with the real client in
    /// `X-Forwarded-For`. The plain helpers above always pass an EMPTY header
    /// map, so they cannot detect a handler that stops forwarding headers —
    /// which is exactly the bug this file's `get_client_ip` exists to fix.
    fn proxied(forwarded: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", forwarded.parse().unwrap());
        h
    }

    async fn request_proxied(state: &InviteState, forwarded: &str) -> StatusCode {
        // The challenge endpoint is rate-limited on the SAME key, so a limited
        // client is refused here rather than at create time. Propagate that
        // status instead of unwrapping — being refused a challenge IS the limit
        // working, and unwrapping would turn the expected outcome into a panic.
        let challenge = match get_invite_challenge(
            State(state.clone()),
            ConnectInfo(addr("127.0.0.1")),
            proxied(forwarded),
        )
        .await
        {
            Ok(r) => r.0.challenge,
            Err((status, _)) => return status,
        };
        let proof = solve(challenge);
        match create_room_invite(
            State(state.clone()),
            ConnectInfo(addr("127.0.0.1")),
            proxied(forwarded),
            Json(proof),
        )
        .await
        {
            Ok(_) => StatusCode::OK,
            Err((code, _)) => code,
        }
    }

    /// Pins the HANDLER WIRING, not just the helper.
    ///
    /// The unit test on `get_client_ip` proves the helper is correct, and the
    /// helper was never the fragile part: the production bug was that every
    /// request keyed on the proxy address, collapsing ALL users into one
    /// bucket. Mutating both call sites to pass `&HeaderMap::new()` reproduces
    /// that bug exactly, and every other test in this module still passes — so
    /// without this test the suite is green on the shipped defect.
    ///
    /// Two distinct forwarded clients must therefore get INDEPENDENT budgets.
    #[tokio::test]
    async fn forwarded_clients_get_independent_budgets_through_the_proxy() {
        let dir = tempfile::tempdir().unwrap();
        // Non-empty exit list: an empty one fails closed with 503, as
        // `missing_tor_list_fails_closed` pins. Neither forwarded client is on it.
        let state = state_with(&dir, &["185.220.101.1"], 100);

        // One client exhausts its own allowance.
        for i in 0..MAX_INVITES_PER_WINDOW {
            assert_eq!(
                request_proxied(&state, "198.51.100.4").await,
                StatusCode::OK,
                "invite {i} for the first forwarded client should be allowed"
            );
        }
        assert_eq!(
            request_proxied(&state, "198.51.100.4").await,
            StatusCode::TOO_MANY_REQUESTS,
            "first forwarded client must be limited after its allowance"
        );

        // A DIFFERENT forwarded client must be unaffected. If the handlers stop
        // passing headers through, both collapse onto the proxy address and
        // this is TOO_MANY_REQUESTS instead.
        assert_eq!(
            request_proxied(&state, "203.0.113.9").await,
            StatusCode::OK,
            "a different forwarded client must have its own budget;              failure here means the handlers are keying on the proxy address"
        );
    }

    /// The challenge endpoint applies the Tor check to the client IP, so behind
    /// the proxy it must apply it to the FORWARDED client. Pinned separately
    /// from the per-IP limit below: they are distinct consumers of `client_ip`
    /// at `get_invite_challenge`, and a single test covering both can be
    /// "repaired" by deleting the half that broke.
    #[tokio::test]
    async fn tor_exit_behind_the_proxy_is_blocked_before_work_is_issued() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &["185.220.101.1"], 100);
        let status = get_invite_challenge(
            State(state.clone()),
            ConnectInfo(addr("127.0.0.1")),
            proxied("185.220.101.1"),
        )
        .await
        .map(|_| StatusCode::OK)
        .unwrap_or_else(|(status, _)| status);
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "a Tor exit arriving via X-Forwarded-For must be refused at the challenge, \
             exactly as tor_is_blocked_before_work_is_issued requires for a direct peer"
        );
        assert_eq!(
            state.global_bucket.current(),
            0,
            "no work should have been issued"
        );
    }

    /// The challenge endpoint's per-IP pre-check must key on the FORWARDED
    /// client. `forwarded_clients_get_independent_budgets_through_the_proxy`
    /// cannot see this: create's own 429 is the same observable status, so it
    /// stays green when the challenge wrongly returns 200.
    #[tokio::test]
    async fn challenge_applies_the_per_ip_limit_to_the_forwarded_client() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &["185.220.101.1"], 100);
        for _ in 0..MAX_INVITES_PER_WINDOW {
            assert_eq!(
                request_proxied(&state, "198.51.100.4").await,
                StatusCode::OK
            );
        }
        let status = get_invite_challenge(
            State(state.clone()),
            ConnectInfo(addr("127.0.0.1")),
            proxied("198.51.100.4"),
        )
        .await
        .map(|_| StatusCode::OK)
        .unwrap_or_else(|(status, _)| status);
        assert_eq!(
            status,
            StatusCode::TOO_MANY_REQUESTS,
            "an exhausted forwarded client must be refused AT THE CHALLENGE, \
             not merely at create"
        );
    }

    #[tokio::test]
    async fn tor_is_blocked_before_work_is_issued() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &["185.220.101.1"], 100);
        assert_eq!(
            challenge(&state, "185.220.101.1").await.unwrap_err(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(state.global_bucket.current(), 0);
    }

    #[tokio::test]
    async fn missing_tor_list_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &[], 100);
        assert_eq!(
            challenge(&state, "203.0.113.1").await.unwrap_err(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn valid_proof_is_required_and_single_use() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &["185.220.101.1"], 100);
        let proof = solve(challenge(&state, "203.0.113.1").await.unwrap());
        let replay = CreateInviteRequest {
            challenge: proof.challenge.clone(),
            nonce: proof.nonce,
        };
        assert_eq!(
            request_with(&state, "203.0.113.1", proof).await,
            StatusCode::OK
        );
        assert_eq!(
            request_with(&state, "203.0.113.1", replay).await,
            StatusCode::CONFLICT
        );
        assert_eq!(state.global_bucket.current(), 1);
    }

    #[tokio::test]
    async fn global_ceiling_holds_across_rotating_non_tor_ips() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &["185.220.101.1"], 3);
        for i in 1..=3 {
            assert_eq!(
                request(&state, &format!("203.0.113.{i}")).await,
                StatusCode::OK
            );
        }
        assert_eq!(
            challenge(&state, "203.0.113.4").await.unwrap_err(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(state.global_bucket.current(), 3);
    }

    #[tokio::test]
    async fn per_ip_rejection_refunds_global_capacity_and_proof() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &["185.220.101.1"], 100);
        for _ in 0..MAX_INVITES_PER_WINDOW {
            assert_eq!(request(&state, "203.0.113.1").await, StatusCode::OK);
        }
        let proof = solve(state.pow.issue(state.global_bucket.current()).challenge);
        assert_eq!(
            request_with(&state, "203.0.113.1", proof).await,
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(state.global_bucket.current(), MAX_INVITES_PER_WINDOW);
    }

    #[test]
    fn startup_seed_excludes_historical_tor_issuance() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("exits.txt");
        std::fs::write(&cache, "185.220.101.1\n").unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        std::fs::write(
            dir.path().join("rl.json"),
            serde_json::json!({
                "invites": {
                    "185.220.101.1": [now.clone()],
                    "203.0.113.1": [now]
                }
            })
            .to_string(),
        )
        .unwrap();
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let state = InviteState::new(
            dir.path().join("rl.json"),
            Some(cache),
            Some(200),
            4,
            signing_key.verifying_key(),
            signing_key,
            "Test Room".to_string(),
        );
        assert_eq!(state.global_bucket.current(), 1);
    }
}
