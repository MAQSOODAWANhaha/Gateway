use gateway_common::entities::certificates::Model as Certificate;
use gateway_common::entities::tls_policies::Model as TlsPolicy;
use gateway_common::snapshot::Snapshot;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CertPaths {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

pub fn materialize_certs(snapshot: &Snapshot, certs_dir: &Path) -> Result<HashMap<Uuid, CertPaths>> {
    fs::create_dir_all(certs_dir)?;
    let mut map = HashMap::new();

    for policy in &snapshot.tls_policies {
        if let Some(cert) = select_cert(policy, &snapshot.certificates) {
            let cert_path = certs_dir.join(format!("{}.pem", policy.id));
            let key_path = certs_dir.join(format!("{}.key", policy.id));
            fs::write(&cert_path, &cert.cert_pem)?;
            fs::write(&key_path, &cert.key_pem)?;
            map.insert(
                policy.id,
                CertPaths {
                    cert_path,
                    key_path,
                },
            );
        }
    }

    Ok(map)
}

fn select_cert<'a>(policy: &TlsPolicy, certs: &'a [Certificate]) -> Option<&'a Certificate> {
    certs
        .iter()
        .filter(|cert| policy.domains.iter().any(|d| d == &cert.domain))
        .max_by_key(|cert| cert.expires_at)
}
