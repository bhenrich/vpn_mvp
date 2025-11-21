use anyhow::{anyhow, Context};
use axum::extract::State;
use axum::{routing::post, Json, Router};
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::info;

#[derive(Clone)]
pub struct AdmissionState {
    pub jwks: Arc<JwkSet>,
    pub redis: redis::Client,
    pub max_active: usize,
    pub session_ttl_secs: usize,
}

#[derive(Deserialize)]
pub struct AdmitRequest {
    pub token: String,
}

static JWKS_CACHE: OnceCell<Arc<JwkSet>> = OnceCell::new();

async fn load_jwks() -> anyhow::Result<Arc<JwkSet>> {
    if let Some(cached) = JWKS_CACHE.get() {
        return Ok(cached.clone());
    }
    let jwks = if let Ok(path) = std::env::var("JWKS_JSON_PATH") {
        let data = std::fs::read_to_string(path).context("reading JWKS file")?;
        serde_json::from_str::<JwkSet>(&data).context("parsing JWKS")?
    } else {
        let url = std::env::var("JWKS_URL").context("JWKS_URL or JWKS_JSON_PATH must be set")?;
        let body = reqwest::get(url)
            .await
            .context("fetching JWKS")?
            .text()
            .await
            .context("reading JWKS body")?;
        serde_json::from_str::<JwkSet>(&body).context("parsing JWKS")?
    };
    let arc = Arc::new(jwks);
    let _ = JWKS_CACHE.set(arc.clone());
    Ok(arc)
}

#[derive(Deserialize)]
struct Claims {
    sub: String, // user id
    did: String, // device id
    jti: String, // session id
    exp: usize,
}

fn jwk_to_decoding_key(jwk: &jsonwebtoken::jwk::Jwk) -> anyhow::Result<DecodingKey> {
    match &jwk.algorithm {
        AlgorithmParameters::RSA(rsa) => Ok(DecodingKey::from_rsa_components(&rsa.n, &rsa.e)?),
        AlgorithmParameters::EllipticCurve(ec) => {
            Ok(DecodingKey::from_ec_components(&ec.x, &ec.y)?)
        }
        _ => Err(anyhow!("unsupported JWK algorithm")),
    }
}

async fn admit(
    State(state): State<AdmissionState>,
    Json(req): Json<AdmitRequest>,
) -> axum::response::Result<Json<serde_json::Value>> {
    let header = decode_header(&req.token).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
    let kid = header.kid.ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
    let alg = header.alg;

    let jwk = state
        .jwks
        .keys
        .iter()
        .find(|k| k.common.key_id.as_deref() == Some(&kid))
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

    let mut validation = Validation::new(Algorithm::from(header.alg));
    validation.set_audience::<&str>(&[]);
    validation.set_issuer::<&str>(&[]);

    let key = jwk_to_decoding_key(jwk).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
    let token_data = decode::<Claims>(&req.token, &key, &validation)
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
    let claims = token_data.claims;

    // Redis counters using ZSET with expiry timestamps
    let mut conn = state
        .redis
        .get_async_connection()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = chrono::Utc::now().timestamp() as f64;
    let expires = now + state.session_ttl_secs as f64;

    let user_key = format!("active:user:{}", claims.sub);
    let device_key = format!("active:device:{}", claims.did);
    // purge expired
    let _: () = redis::cmd("ZREMRANGEBYSCORE")
        .arg(&user_key)
        .arg("-inf")
        .arg(now)
        .query_async(&mut conn)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let _: () = redis::cmd("ZREMRANGEBYSCORE")
        .arg(&device_key)
        .arg("-inf")
        .arg(now)
        .query_async(&mut conn)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    // counts
    let user_count: isize = redis::cmd("ZCARD")
        .arg(&user_key)
        .query_async(&mut conn)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let device_count: isize = redis::cmd("ZCARD")
        .arg(&device_key)
        .query_async(&mut conn)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    if (user_count as usize) >= state.max_active {
        return Err(axum::http::StatusCode::TOO_MANY_REQUESTS.into());
    }

    // add session
    let _: () = redis::cmd("ZADD")
        .arg(&user_key)
        .arg(expires)
        .arg(&claims.jti)
        .query_async(&mut conn)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let _: () = redis::cmd("ZADD")
        .arg(&device_key)
        .arg(expires)
        .arg(&claims.jti)
        .query_async(&mut conn)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"allowed": true, "expires_at": expires })))
}

pub async fn run_admission_server() -> anyhow::Result<()> {
    let jwks = load_jwks().await?;
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let client = redis::Client::open(redis_url)?;
    let max_active = std::env::var("MAX_ACTIVE_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let session_ttl_secs = std::env::var("CONNECTION_TTL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    let state = AdmissionState {
        jwks,
        redis: client,
        max_active,
        session_ttl_secs,
    };
    // Clone before moving state into the app
    let redis_for_reaper = state.redis.clone();
    let app = Router::new().route("/admit", post(admit)).with_state(state);
    let addr = std::env::var("ADMISSION_ADDR").unwrap_or_else(|_| "0.0.0.0:9090".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tokio::spawn(async move {
        loop {
            if let Err(err) = reap_expired(redis_for_reaper.clone()).await {
                tracing::warn!("reaper error: {}", err);
            }
            sleep(Duration::from_secs(30)).await;
        }
    });
    axum::serve(listener, app).await?;
    Ok(())
}

async fn reap_expired(client: redis::Client) -> anyhow::Result<()> {
    let mut conn = client.get_async_connection().await?;
    // Find active keys (dev-only approach; production should track keys explicitly)
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("active:*")
        .query_async(&mut conn)
        .await?;
    let now = chrono::Utc::now().timestamp() as f64;
    for key in keys {
        let _: () = redis::cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg("-inf")
            .arg(now)
            .query_async(&mut conn)
            .await?;
    }
    Ok(())
}
