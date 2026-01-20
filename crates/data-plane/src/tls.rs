use anyhow::Result;
use gateway_common::entities::certificates::Model as Certificate;
use gateway_common::entities::tls_policies::Model as TlsPolicy;
use gateway_common::snapshot::Snapshot;
use rcgen::CertificateParams;
use std::fs;
use std::path::Path;
use uuid::Uuid;

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

fn read_default_cert_if_present(certs_dir: &Path) -> Result<Option<TlsKeyPairPem>> {
    let cert_path = certs_dir.join("default.pem");
    let key_path = certs_dir.join("default.key");
    if !cert_path.exists() || !key_path.exists() {
        return Ok(None);
    }
    Ok(Some(TlsKeyPairPem {
        cert_pem: fs::read(cert_path)?,
        key_pem: fs::read(key_path)?,
    }))
}

pub fn default_tls_pem(certs_dir: &Path) -> Result<TlsKeyPairPem> {
    if let Some(pem) = read_default_cert_if_present(certs_dir)? {
        return Ok(pem);
    }
    let mut params = CertificateParams::new(vec!["gateway.local".to_string()])?;
    params.is_ca = rcgen::IsCa::NoCa;
    let key_pair = rcgen::KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    Ok(TlsKeyPairPem {
        cert_pem: cert.pem().as_bytes().to_vec(),
        key_pem: key_pair.serialize_pem().as_bytes().to_vec(),
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
