use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use thiserror::Error;

use crate::config::Config;
use crate::encoder::Encoder;
use crate::gts::{parse_line, Datum};
use crate::ids::{class_id, gts_ids_key, labels_id};
use crate::kafka::Producer;
use crate::policy::{resolve_labels, PolicyError};
use crate::thrift::{KafkaDataMessage, Metadata, KAFKA_MSG_STORE};

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("invalid token")]
    InvalidToken,
    #[error("policy error: {0}")]
    Policy(String),
    #[error("parse error on line {line}: {err}")]
    Parse { line: usize, err: String },
    #[error("kafka error: {0}")]
    Kafka(String),
}

pub struct AppState {
    pub cfg: Config,
    pub producer: Arc<dyn Producer>,
    pub seen_meta: Mutex<HashSet<(i64, i64)>>,
}

pub fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64
}

pub fn process_update(
    state: &AppState,
    token: &str,
    body: &str,
) -> Result<usize, UpdateError> {
    let claims = state
        .cfg
        .tokens
        .get(token)
        .ok_or(UpdateError::InvalidToken)?;
    let now = now_micros();
    let mut encoders: HashMap<(i64, i64), (String, BTreeMap<String, String>, Encoder)> =
        HashMap::new();
    let mut count = 0usize;
    for (i, raw) in body.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let datum: Datum = parse_line(raw)
            .map_err(|e| UpdateError::Parse {
                line: i + 1,
                err: e.to_string(),
            })?;
        let labels = resolve_labels(claims, &datum.labels, &state.cfg.policy)
            .map_err(|e| match e {
                PolicyError::ForbiddenOwner => UpdateError::Policy("owner".to_string()),
                PolicyError::ForbiddenProducer => UpdateError::Policy("producer".to_string()),
            })?;
        let cid = class_id(state.cfg.class_key, &datum.class);
        let lid = labels_id(state.cfg.labels_key, &labels);
        let entry = encoders
            .entry((cid, lid))
            .or_insert_with(|| (datum.class.clone(), labels.clone(), Encoder::new(now)));
        entry.2.add(datum.ts, datum.location, datum.elevation, &datum.value);
        count += 1;
    }
    for ((cid, lid), (name, labels, enc)) in encoders {
        let key = gts_ids_key(cid, lid);
        let data = enc.into_bytes();
        let msg = KafkaDataMessage {
            class_id: cid,
            labels_id: lid,
            data: Some(data),
            msg_type: KAFKA_MSG_STORE,
            metadata: None,
            attributes: None,
        };
        state
            .producer
            .send(&state.cfg.data_topic, &key, &msg.serialize())
            .map_err(|e| UpdateError::Kafka(e.to_string()))?;
        let mut seen = state.seen_meta.lock().unwrap();
        if seen.insert((cid, lid)) {
            let meta = Metadata {
                name,
                labels,
                class_id: cid,
                labels_id: lid,
                attributes: None,
            };
            let mut value = key.to_vec();
            value.extend_from_slice(&meta.serialize());
            state
                .producer
                .send(&state.cfg.meta_topic, &key, &value)
                .map_err(|e| UpdateError::Kafka(e.to_string()))?;
        }
    }
    Ok(count)
}

pub async fn update_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    let token = match headers.get("x-warp10-token").and_then(|v| v.to_str().ok()) {
        Some(t) => t.to_string(),
        None => {
            return (
                StatusCode::FORBIDDEN,
                "missing X-Warp10-Token header".to_string(),
            )
                .into_response();
        }
    };
    match process_update(&state, &token, &body) {
        Ok(n) => (StatusCode::OK, format!("{} datapoints\n", n)).into_response(),
        Err(UpdateError::InvalidToken) => (StatusCode::FORBIDDEN, "Invalid token.".to_string()).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v0/update", post(update_handler))
        .route("/healthz", get(healthz))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kafka::MockProducer;
    use crate::policy::{PolicyConfig, TokenClaims};

    fn test_state(producer: Arc<MockProducer>, claims: TokenClaims) -> Arc<AppState> {
        let mut tokens = HashMap::new();
        tokens.insert("tok".to_string(), claims);
        Arc::new(AppState {
            cfg: Config {
                listen: "127.0.0.1:0".to_string(),
                kafka_brokers: "unused".to_string(),
                data_topic: "data".to_string(),
                meta_topic: "metadata".to_string(),
                class_key: (0x0706050403020100, 0x0f0e0d0c0b0a0908),
                labels_key: (0x0706050403020100, 0x0f0e0d0c0b0a0908),
                tokens,
                policy: PolicyConfig::default(),
                use_kafka: false,
            },
            producer: producer.clone(),
            seen_meta: Mutex::new(HashSet::new()),
        })
    }

    fn claims_with_owner() -> TokenClaims {
        TokenClaims {
            owner: Some("11111111-1111-1111-1111-111111111111".to_string()),
            producer: Some("22222222-2222-2222-2222-222222222222".to_string()),
            app: Some("app".to_string()),
            labels: BTreeMap::new(),
            attributes: HashSet::new(),
        }
    }

    #[test]
    fn update_publishes_data_and_metadata() {
        let producer = Arc::new(MockProducer::new());
        let state = test_state(producer.clone(), claims_with_owner());
        let body = "1000// foo{a=b} 42\n2000// foo{a=b} 43\n1500// bar{} true\n";
        let n = process_update(&state, "tok", body).unwrap();
        assert_eq!(n, 3);
        let sent = producer.sent.lock().unwrap();
        assert_eq!(sent.len(), 4);
        assert_eq!(sent[0].topic, "data");
        assert_eq!(sent[1].topic, "metadata");
        assert_eq!(sent[2].topic, "data");
        assert_eq!(sent[3].topic, "metadata");
        assert_eq!(sent[0].key.len(), 16);
        assert_eq!(sent[1].key.len(), 16);
        assert!(sent[1].value.len() > 16);
    }

    #[test]
    fn update_rejects_unknown_token() {
        let producer = Arc::new(MockProducer::new());
        let state = test_state(producer, claims_with_owner());
        assert!(matches!(
            process_update(&state, "nope", "1// c{} 1"),
            Err(UpdateError::InvalidToken)
        ));
    }

    #[tokio::test]
    async fn http_update_endpoint_end_to_end() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let producer = Arc::new(MockProducer::new());
        let state = test_state(producer.clone(), claims_with_owner());
        let app = router(state);
        let req = Request::builder()
            .method("POST")
            .uri("/api/v0/update")
            .header("x-warp10-token", "tok")
            .body(Body::from("1000// foo{a=b} 42\n"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let req = Request::builder()
            .method("POST")
            .uri("/api/v0/update")
            .header("x-warp10-token", "bad")
            .body(Body::from("1// c{} 1\n"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(!producer.sent.lock().unwrap().is_empty());
    }

    #[test]
    fn client_owner_used_when_token_allows() {
        let producer = Arc::new(MockProducer::new());
        let mut c = claims_with_owner();
        c.owner = None;
        c.attributes.insert(".allowowner".to_string());
        let state = test_state(producer.clone(), c);
        let body = "1// foo{.owner=99999999-9999-9999-9999-999999999999} 1\n";
        process_update(&state, "tok", body).unwrap();
        let sent = producer.sent.lock().unwrap();
        let meta = &sent[1].value[16..];
        let s = String::from_utf8_lossy(meta);
        assert!(s.contains("99999999-9999-9999-9999-999999999999"));
        assert!(!s.contains("11111111-1111-1111-1111-111111111111"));
    }
}
