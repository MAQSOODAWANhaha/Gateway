use anyhow::Result;
use gateway_common::entities::certificates::Model as Certificate;
use gateway_common::entities::tls_policies::Model as TlsPolicy;
use gateway_common::snapshot::Snapshot;
use rcgen::CertificateParams;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct CertPaths {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TlsKeyPairPem {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
}

fn select_cert<'a>(policy: &TlsPolicy, certs: &'a [Certificate]) -> Option<&'a Certificate> {
    certs
        .iter()
        .filter(|cert| policy.domains.iter().any(|d| d == &cert.domain))
        .max_by_key(|cert| cert.expires_at)
}

fn ensure_default_cert(certs_dir: &Path) -> Result<CertPaths> {
    let cert_path = certs_dir.join("default.pem");
    let key_path = certs_dir.join("default.key");
    if cert_path.exists() && key_path.exists() {
        return Ok(CertPaths {
            cert_path,
            key_path,
        });
    }

    let mut params = CertificateParams::new(vec!["gateway.local".to_string()])?;
    params.is_ca = rcgen::IsCa::NoCa;
    let key_pair = rcgen::KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    fs::write(&cert_path, cert.pem())?;
    fs::write(&key_path, key_pair.serialize_pem())?;
    Ok(CertPaths {
        cert_path,
        key_path,
    })
}

pub fn default_tls_pem(certs_dir: &Path) -> Result<TlsKeyPairPem> {
    let paths = ensure_default_cert(certs_dir)?;
    Ok(TlsKeyPairPem {
        cert_pem: fs::read(paths.cert_path)?,
        key_pem: fs::read(paths.key_path)?,
    })
}

pub fn tls_pem_for_policy(snapshot: &Snapshot, policy_id: Uuid) -> Option<TlsKeyPairPem> {
    let policy = snapshot.tls_policies.iter().find(|p| p.id == policy_id)?;
    let cert = select_cert(policy, &snapshot.certificates)?;
    Some(TlsKeyPairPem {
        cert_pem: cert.cert_pem.as_bytes().to_vec(),
        key_pem: cert.key_pem.as_bytes().to_vec(),
    })
}
