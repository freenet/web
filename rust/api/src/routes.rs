use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Path, State},
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
use crate::invite_blocklist::{BanReport, InviteBlocklist};
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
    /// Source addresses of banned members, refused for a week. See
    /// `invite_blocklist` for why this is address-scoped and time-boxed.
    pub blocklist: Arc<InviteBlocklist>,
    /// Shared secret authorising ban reports. `None` disables the endpoint,
    /// which is the correct posture when no secret is configured: an
    /// unauthenticated version would let anyone block any address.
    pub ban_report_token: Option<String>,
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
    // Grew past clippy's threshold when the blocklist path and operator token
    // were threaded through. Bundling these into a config struct is a wider
    // refactor of every caller than this change warrants.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rate_limit_file: PathBuf,
        blocklist_file: PathBuf,
        ban_report_token: Option<String>,
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
            blocklist: Arc::new(InviteBlocklist::new(blocklist_file)),
            ban_report_token,
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
    if state.blocklist.is_blocked(client_ip) {
        warn!("Invite request refused from blocked source: {}", client_ip);
        return Err(invite_error(
            StatusCode::FORBIDDEN,
            "Invitations are not available from this network right now. \
             If you are on a VPN, try again without it, or ask someone in the \
             room for an invite link.",
            None,
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
) -> Result<Json<PowChallengeResponse>, (StatusCode, Json<InviteErrorResponse>)> {
    let client_ip = get_client_ip(addr);
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
    Json(request): Json<CreateInviteRequest>,
) -> Result<Json<CreateInviteResponse>, (StatusCode, Json<InviteErrorResponse>)> {
    let client_ip = get_client_ip(addr);
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
            // Best effort: a member we cannot map is one we cannot act on
            // later, but failing issuance over it would be worse.
            if let Err(e) = state.blocklist.record_source(&created.member_id, client_ip) {
                warn!(
                    "Could not record invite source for {}: {e}",
                    created.member_id
                );
            }
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

#[derive(Deserialize)]
pub struct ReportBanRequest {
    pub member_id: String,
}

#[derive(Serialize)]
pub struct ReportBanResponse {
    pub outcome: String,
}

/// Block the source address behind a banned member.
///
/// Authenticated with a shared secret, because an open version of this would
/// let anyone deny invites to any address by naming a member they did not ban.
/// Absent a configured secret the endpoint refuses everything.
fn authorize_operator(
    state: &InviteState,
    headers: &axum::http::HeaderMap,
) -> Result<(), (StatusCode, Json<InviteErrorResponse>)> {
    let Some(expected) = state.ban_report_token.as_deref() else {
        warn!("Operator request refused: no token configured");
        return Err(invite_error(StatusCode::NOT_FOUND, "Not found.", None));
    };
    let presented = headers
        .get("x-ban-report-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    // Constant-time compare so the token cannot be recovered byte by byte.
    let authorized = presented.len() == expected.len()
        && presented
            .as_bytes()
            .iter()
            .zip(expected.as_bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0;
    if !authorized {
        warn!("Operator request refused: bad token");
        return Err(invite_error(
            StatusCode::UNAUTHORIZED,
            "Unauthorized.",
            None,
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct BlockRequest {
    pub ip: String,
    pub reason: Option<String>,
}

/// Block an address directly. Same week-long duration as a ban-driven block.
async fn block_source(
    State(state): State<InviteState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<BlockRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<InviteErrorResponse>)> {
    authorize_operator(&state, &headers)?;
    let Ok(ip) = request.ip.parse::<IpAddr>() else {
        return Err(invite_error(
            StatusCode::BAD_REQUEST,
            "Malformed address.",
            None,
        ));
    };
    let reason = request.reason.as_deref().unwrap_or("manual operator block");
    match state.blocklist.block_ip(ip, reason) {
        Ok(until) => {
            info!("Operator blocked invite source {ip} until {until} ({reason})");
            Ok(Json(serde_json::json!({
                "ip": ip.to_string(),
                "blocked_until": until.to_rfc3339(),
            })))
        }
        Err(e) => {
            error!("Blocklist error handling manual block: {e}");
            Err(invite_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error.",
                None,
            ))
        }
    }
}

#[derive(Deserialize)]
pub struct UnblockRequest {
    pub ip: String,
}

/// Lift a block early, and list what remains.
///
/// This exists because a blocked address can be a shared VPN exit, so an
/// operator needs to undo a block that caught the wrong people without waiting
/// out the week or restarting the service.
async fn unblock_source(
    State(state): State<InviteState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<UnblockRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<InviteErrorResponse>)> {
    authorize_operator(&state, &headers)?;
    let Ok(ip) = request.ip.parse::<IpAddr>() else {
        return Err(invite_error(
            StatusCode::BAD_REQUEST,
            "Malformed address.",
            None,
        ));
    };
    match state.blocklist.unblock(ip) {
        Ok(removed) => {
            info!("Operator unblock of {ip}: removed={removed}");
            let remaining: Vec<_> = state
                .blocklist
                .active_blocks()
                .into_iter()
                .map(|(ip, until, member_id)| {
                    serde_json::json!({
                        "ip": ip.to_string(),
                        "blocked_until": until.to_rfc3339(),
                        "member_id": member_id,
                    })
                })
                .collect();
            Ok(Json(serde_json::json!({
                "removed": removed,
                "active_blocks": remaining,
            })))
        }
        Err(e) => {
            error!("Blocklist error handling unblock: {e}");
            Err(invite_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error.",
                None,
            ))
        }
    }
}

async fn report_ban(
    State(state): State<InviteState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<ReportBanRequest>,
) -> Result<Json<ReportBanResponse>, (StatusCode, Json<InviteErrorResponse>)> {
    authorize_operator(&state, &headers)?;

    match state.blocklist.report_ban(&request.member_id) {
        Ok(BanReport::Blocked { ip, until }) => {
            info!(
                "Blocked invite source {} until {} after ban of member {}",
                ip, until, request.member_id
            );
            Ok(Json(ReportBanResponse {
                outcome: "blocked".into(),
            }))
        }
        Ok(BanReport::Extended { ip, until }) => {
            info!(
                "Extended block on invite source {} to {} after ban of member {}",
                ip, until, request.member_id
            );
            Ok(Json(ReportBanResponse {
                outcome: "extended".into(),
            }))
        }
        Ok(BanReport::UnknownMember) => {
            info!(
                "Ban reported for member {} with no recorded invite source",
                request.member_id
            );
            Ok(Json(ReportBanResponse {
                outcome: "unknown_member".into(),
            }))
        }
        Err(e) => {
            error!("Blocklist error handling ban report: {e}");
            Err(invite_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error.",
                None,
            ))
        }
    }
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
        .route("/report-ban", post(report_ban))
        .route("/block-source", post(block_source))
        .route("/unblock-source", post(unblock_source))
        .with_state(state)
        .layer(cors)
}

#[cfg(test)]
mod invite_handler_tests {
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
            blocklist: Arc::new(InviteBlocklist::new(dir.path().join("blocklist.json"))),
            ban_report_token: Some("test-token".to_string()),
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
        get_invite_challenge(State(state.clone()), ConnectInfo(addr(ip)))
            .await
            .map(|response| response.0.challenge)
            .map_err(|(status, _)| status)
    }

    async fn request_with(
        state: &InviteState,
        ip: &str,
        request: CreateInviteRequest,
    ) -> StatusCode {
        match create_room_invite(State(state.clone()), ConnectInfo(addr(ip)), Json(request)).await {
            Ok(_) => StatusCode::OK,
            Err((code, _)) => code,
        }
    }

    async fn request(state: &InviteState, ip: &str) -> StatusCode {
        let proof = solve(challenge(state, ip).await.unwrap());
        request_with(state, ip, proof).await
    }

    /// The whole point of the module, exercised through the handlers: an
    /// address mints an invite, its member is banned, and the same address is
    /// refused before it can spend any work. This is the 2026-07-26 sequence,
    /// where the same address came back 37 minutes after a ban and succeeded.
    #[tokio::test]
    async fn a_banned_members_source_is_refused_on_its_next_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with(&dir, &["185.220.101.1"], 100);
        let source = "170.62.100.54";

        assert_eq!(request(&state, source).await, StatusCode::OK);

        // The handler records the member behind that invite; take it from the
        // ledger rather than hardcoding a generated id.
        let member_id = state
            .blocklist
            .active_blocks()
            .first()
            .map(|(_, _, member)| member.clone());
        assert!(member_id.is_none(), "nothing should be blocked yet");

        // Report the ban for whichever member that invite created.
        let minted = state.blocklist.recorded_members();
        assert_eq!(minted.len(), 1, "the invite should have recorded a source");
        assert!(matches!(
            state.blocklist.report_ban(&minted[0]).unwrap(),
            BanReport::Blocked { .. }
        ));

        // Refused at the network gate, before proof of work is even issued.
        assert_eq!(
            challenge(&state, source).await.unwrap_err(),
            StatusCode::FORBIDDEN
        );
        // An unrelated address is unaffected.
        assert!(challenge(&state, "73.11.36.49").await.is_ok());
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
            dir.path().join("blocklist.json"),
            None,
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
