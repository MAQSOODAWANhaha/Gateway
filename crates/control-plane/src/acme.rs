use gateway_common::config::AppConfig;
use gateway_common::entities::{acme_accounts, certificates, tls_policies};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, NewAccount,
    NewOrder, OrderStatus, RetryPolicy,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;
use x509_parser::pem::parse_x509_pem;

#[derive(Clone, Default)]
pub struct AcmeChallengeStore {
    inner: Arc<RwLock<HashMap<String, String>>>,
}

impl AcmeChallengeStore {
    pub async fn set(&self, token: String, key_auth: String) {
        self.inner.write().await.insert(token, key_auth);
    }

    pub async fn get(&self, token: &str) -> Option<String> {
        self.inner.read().await.get(token).cloned()
    }

    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
}

pub async fn run_acme_worker(
    db: DatabaseConnection,
    store: AcmeChallengeStore,
    config: AppConfig,
) -> Result<()> {
    if !config.acme_enabled {
        info!("ACME disabled, skipping worker");
        return Ok(());
    }

    let contact_email = match &config.acme_contact_email {
        Some(email) => email.clone(),
        None => return Err(anyhow!("ACME_CONTACT_EMAIL is required when ACME is enabled")),
    };

    fs::create_dir_all(&config.acme_storage_dir)?;

    loop {
        if let Err(err) = process_acme_policies(&db, &store, &config, &contact_email).await {
            error!("acme worker error: {}", err);
        }
        sleep(std::time::Duration::from_secs(300)).await;
    }
}

async fn process_acme_policies(
    db: &DatabaseConnection,
    store: &AcmeChallengeStore,
    config: &AppConfig,
    contact_email: &str,
) -> Result<()> {
    let policies = tls_policies::Entity::find()
        .filter(tls_policies::Column::Mode.eq("auto"))
        .all(db)
        .await?;

    if policies.is_empty() {
        return Ok(());
    }

    let account = load_or_create_account(db, config, contact_email).await?;

    for policy in policies {
        if policy.domains.is_empty() {
            update_tls_status(db, policy.id, "error").await?;
            continue;
        }

        if let Some(expiry) = latest_cert_expiry(db, &policy.domains).await? {
            let renew_at = expiry - Duration::days(30);
            if Utc::now() < renew_at {
                continue;
            }
        }

        match order_certificate(&account, store, &policy.domains).await {
            Ok((cert_pem, key_pem, expires_at)) => {
                store_certificate(db, &policy.domains[0], cert_pem, key_pem, expires_at).await?;
                update_tls_status(db, policy.id, "active").await?;
            }
            Err(err) => {
                warn!("acme order failed for policy {}: {}", policy.id, err);
                update_tls_status(db, policy.id, "error").await?;
            }
        }
    }

    Ok(())
}

async fn load_or_create_account(
    db: &DatabaseConnection,
    config: &AppConfig,
    contact_email: &str,
) -> Result<Account> {
    let account = acme_accounts::Entity::find()
        .filter(acme_accounts::Column::DirectoryUrl.eq(&config.acme_directory_url))
        .order_by_desc(acme_accounts::Column::CreatedAt)
        .one(db)
        .await?;

    if let Some(account) = account {
        let creds: AccountCredentials = serde_json::from_value(account.credentials_json)?;
        return Ok(Account::builder()?.from_credentials(creds).await?);
    }

    let (account, creds) = Account::builder()?
        .create(
            &NewAccount {
                contact: &[contact_email],
                terms_of_service_agreed: true,
                only_return_existing: false,
            },
            config.acme_directory_url.clone(),
            None,
        )
        .await?;

    let json = serde_json::to_value(&creds)?;
    let active = acme_accounts::ActiveModel {
        id: Set(Uuid::new_v4()),
        directory_url: Set(config.acme_directory_url.clone()),
        credentials_json: Set(json),
        ..Default::default()
    };
    active.insert(db).await?;

    Ok(account)
}

async fn order_certificate(
    account: &Account,
    store: &AcmeChallengeStore,
    domains: &[String],
) -> Result<(String, String, DateTime<Utc>)> {
    let result = order_certificate_inner(account, store, domains).await;
    store.clear().await;
    result
}

async fn order_certificate_inner(
    account: &Account,
    store: &AcmeChallengeStore,
    domains: &[String],
) -> Result<(String, String, DateTime<Utc>)> {
    let identifiers: Vec<Identifier> = domains
        .iter()
        .map(|d| Identifier::Dns(d.clone()))
        .collect();
    let mut order = account
        .new_order(&NewOrder::new(&identifiers))
        .await?;

    let mut authorizations = order.authorizations();
    while let Some(result) = authorizations.next().await {
        let mut authz = result?;
        if authz.status == AuthorizationStatus::Valid {
            continue;
        }

        let mut challenge = authz
            .challenge(ChallengeType::Http01)
            .ok_or_else(|| anyhow!("missing HTTP-01 challenge"))?;

        let key_auth = challenge.key_authorization().as_str().to_string();
        let token = challenge.token.clone();

        store.set(token.clone(), key_auth).await;
        challenge.set_ready().await?;
    }

    let status = order.poll_ready(&RetryPolicy::default()).await?;
    if status != OrderStatus::Ready {
        return Err(anyhow!("order not ready: {:?}", status));
    }

    let private_key_pem = order.finalize().await?;
    let cert_chain_pem = order.poll_certificate(&RetryPolicy::default()).await?;

    let expires_at = parse_cert_expiry(&cert_chain_pem)?;
    Ok((cert_chain_pem, private_key_pem, expires_at))
}

async fn latest_cert_expiry(
    db: &DatabaseConnection,
    domains: &[String],
) -> Result<Option<DateTime<Utc>>> {
    for domain in domains {
        let row = certificates::Entity::find()
            .filter(certificates::Column::Domain.eq(domain))
            .order_by_desc(certificates::Column::ExpiresAt)
            .one(db)
            .await?;
        if let Some(model) = row {
            return Ok(Some(model.expires_at.into()));
        }
    }
    Ok(None)
}

async fn store_certificate(
    db: &DatabaseConnection,
    domain: &str,
    cert_pem: String,
    key_pem: String,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    let active = certificates::ActiveModel {
        id: Set(Uuid::new_v4()),
        domain: Set(domain.to_string()),
        cert_pem: Set(cert_pem),
        key_pem: Set(key_pem),
        expires_at: Set(expires_at.into()),
        status: Set("active".to_string()),
        ..Default::default()
    };
    active.insert(db).await?;
    Ok(())
}

async fn update_tls_status(db: &DatabaseConnection, id: Uuid, status: &str) -> Result<()> {
    if let Some(policy) = tls_policies::Entity::find_by_id(id).one(db).await? {
        let mut active: tls_policies::ActiveModel = policy.into();
        active.status = Set(status.to_string());
        active.update(db).await?;
    }
    Ok(())
}

fn parse_cert_expiry(cert_pem: &str) -> Result<DateTime<Utc>> {
    let (_, pem) = parse_x509_pem(cert_pem.as_bytes())?;
    let cert = pem.parse_x509()?;
    let not_after = cert.validity().not_after.to_datetime();
    let timestamp = not_after.unix_timestamp();
    let nanos = not_after.nanosecond();
    let dt = DateTime::<Utc>::from_timestamp(timestamp, nanos)
        .ok_or_else(|| anyhow!("invalid certificate timestamp"))?;
    Ok(dt)
}
