use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;
use thiserror::Error;

pub const OWNER_LABEL: &str = ".owner";
pub const PRODUCER_LABEL: &str = ".producer";
pub const APP_LABEL: &str = ".app";

pub const DEFAULT_ALLOW_OWNER_ATTR: &str = ".allowowner";
pub const DEFAULT_ALLOW_PRODUCER_ATTR: &str = ".allowproducer";

#[derive(Debug, Clone, Deserialize)]
pub struct TokenClaims {
    pub owner: Option<String>,
    pub producer: Option<String>,
    pub app: Option<String>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub attributes: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingBehavior {
    Reject,
    Default,
}

#[derive(Debug, Clone)]
pub struct PolicyConfig {
    pub default_owner: String,
    pub default_producer: String,
    pub missing_owner: MissingBehavior,
    pub missing_producer: MissingBehavior,
    pub allow_owner_attr: String,
    pub allow_producer_attr: String,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            default_owner: "00000000-0000-0000-0000-000000000000".to_string(),
            default_producer: "00000000-0000-0000-0000-000000000000".to_string(),
            missing_owner: MissingBehavior::Reject,
            missing_producer: MissingBehavior::Reject,
            allow_owner_attr: DEFAULT_ALLOW_OWNER_ATTR.to_string(),
            allow_producer_attr: DEFAULT_ALLOW_PRODUCER_ATTR.to_string(),
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum PolicyError {
    #[error("forbidden: token has no owner and client .owner is not allowed")]
    ForbiddenOwner,
    #[error("forbidden: token has no producer and client .producer is not allowed")]
    ForbiddenProducer,
}

fn resolve_internal(
    token_value: Option<&str>,
    client_value: Option<&str>,
    allowed: bool,
    missing: MissingBehavior,
    default: &str,
    forbidden: PolicyError,
) -> Result<String, PolicyError> {
    if let Some(v) = token_value {
        return Ok(v.to_string());
    }
    if allowed {
        if let Some(v) = client_value {
            return Ok(v.to_string());
        }
    }
    match missing {
        MissingBehavior::Default => Ok(default.to_string()),
        MissingBehavior::Reject => Err(forbidden),
    }
}

pub fn resolve_labels(
    claims: &TokenClaims,
    client_labels: &BTreeMap<String, String>,
    cfg: &PolicyConfig,
) -> Result<BTreeMap<String, String>, PolicyError> {
    let mut labels = client_labels.clone();
    for (k, v) in &claims.labels {
        if k == OWNER_LABEL || k == PRODUCER_LABEL || k == APP_LABEL {
            continue;
        }
        labels.insert(k.clone(), v.clone());
    }
    let owner = resolve_internal(
        claims.owner.as_deref(),
        client_labels.get(OWNER_LABEL).map(|s| s.as_str()),
        claims.attributes.contains(&cfg.allow_owner_attr),
        cfg.missing_owner,
        &cfg.default_owner,
        PolicyError::ForbiddenOwner,
    )?;
    let producer = resolve_internal(
        claims.producer.as_deref(),
        client_labels.get(PRODUCER_LABEL).map(|s| s.as_str()),
        claims.attributes.contains(&cfg.allow_producer_attr),
        cfg.missing_producer,
        &cfg.default_producer,
        PolicyError::ForbiddenProducer,
    )?;
    labels.insert(OWNER_LABEL.to_string(), owner);
    labels.insert(PRODUCER_LABEL.to_string(), producer);
    match &claims.app {
        Some(app) => {
            labels.insert(APP_LABEL.to_string(), app.clone());
        }
        None => {
            if let Some(app) = client_labels.get(APP_LABEL) {
                labels.insert(APP_LABEL.to_string(), app.clone());
            } else {
                labels.remove(APP_LABEL);
            }
        }
    }
    Ok(labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(owner: Option<&str>, producer: Option<&str>, attrs: &[&str]) -> TokenClaims {
        TokenClaims {
            owner: owner.map(|s| s.to_string()),
            producer: producer.map(|s| s.to_string()),
            app: Some("myapp".to_string()),
            labels: BTreeMap::new(),
            attributes: attrs.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn client(labels: &[(&str, &str)]) -> BTreeMap<String, String> {
        labels
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn token_owner_producer_are_forced() {
        let c = claims(Some("owner-token"), Some("producer-token"), &[]);
        let cl = client(&[(OWNER_LABEL, "owner-client"), (PRODUCER_LABEL, "producer-client"), ("x", "1")]);
        let out = resolve_labels(&c, &cl, &PolicyConfig::default()).unwrap();
        assert_eq!(out.get(OWNER_LABEL).unwrap(), "owner-token");
        assert_eq!(out.get(PRODUCER_LABEL).unwrap(), "producer-token");
        assert_eq!(out.get("x").unwrap(), "1");
        assert_eq!(out.get(APP_LABEL).unwrap(), "myapp");
    }

    #[test]
    fn client_owner_allowed_with_attribute() {
        let c = claims(None, Some("p"), &[DEFAULT_ALLOW_OWNER_ATTR]);
        let cl = client(&[(OWNER_LABEL, "owner-client")]);
        let cfg = PolicyConfig {
            missing_owner: MissingBehavior::Reject,
            ..Default::default()
        };
        let out = resolve_labels(&c, &cl, &cfg).unwrap();
        assert_eq!(out.get(OWNER_LABEL).unwrap(), "owner-client");
    }

    #[test]
    fn client_owner_rejected_without_attribute() {
        let c = claims(None, Some("p"), &[]);
        let cl = client(&[(OWNER_LABEL, "owner-client")]);
        let cfg = PolicyConfig {
            missing_owner: MissingBehavior::Reject,
            ..Default::default()
        };
        assert_eq!(
            resolve_labels(&c, &cl, &cfg),
            Err(PolicyError::ForbiddenOwner)
        );
    }

    #[test]
    fn missing_owner_falls_back_to_default_when_configured() {
        let c = claims(None, Some("p"), &[]);
        let cl = client(&[]);
        let cfg = PolicyConfig {
            missing_owner: MissingBehavior::Default,
            default_owner: "default-owner".to_string(),
            ..Default::default()
        };
        let out = resolve_labels(&c, &cl, &cfg).unwrap();
        assert_eq!(out.get(OWNER_LABEL).unwrap(), "default-owner");
    }

    #[test]
    fn token_labels_cannot_set_internal_labels() {
        let mut c = claims(Some("o"), Some("p"), &[]);
        c.labels.insert(OWNER_LABEL.to_string(), "evil".to_string());
        c.labels.insert("env".to_string(), "prod".to_string());
        let out = resolve_labels(&c, &client(&[]), &PolicyConfig::default()).unwrap();
        assert_eq!(out.get(OWNER_LABEL).unwrap(), "o");
        assert_eq!(out.get("env").unwrap(), "prod");
    }
}
