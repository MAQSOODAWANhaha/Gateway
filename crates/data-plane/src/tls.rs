use anyhow::Result;
use gateway_common::entities::certificates::Model as Certificate;
use gateway_common::entities::tls_policies::Model as TlsPolicy;
use gateway_common::snapshot::Snapshot;
use rcgen::CertificateParams;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CertPaths {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

pub fn materialize_certs(
    snapshot: &Snapshot,
    certs_dir: &Path,
) -> Result<HashMap<Uuid, CertPaths>> {
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

pub fn materialize_https_port_certs(
    snapshot: &Snapshot,
    certs_dir: &Path,
    https_ports: impl Iterator<Item = u16>,
    policy_certs: &HashMap<Uuid, CertPaths>,
) -> Result<HashMap<u16, CertPaths>> {
    fs::create_dir_all(certs_dir)?;
    let default_paths = ensure_default_cert(certs_dir)?;
    let default_cert = fs::read(&default_paths.cert_path)?;
    let default_key = fs::read(&default_paths.key_path)?;

    let mut listeners_by_port: HashMap<u16, &gateway_common::entities::listeners::Model> =
        HashMap::new();
    for listener in &snapshot.listeners {
        if !listener.enabled {
            continue;
        }
        if !(1..=65535).contains(&listener.port) {
            continue;
        }
        listeners_by_port.insert(listener.port as u16, listener);
    }

    let mut map = HashMap::new();
    for port in https_ports {
        let (cert_bytes, key_bytes) = match listeners_by_port.get(&port) {
            Some(listener)
                if listener.protocol.eq_ignore_ascii_case("https")
                    && listener
                        .tls_policy_id
                        .and_then(|id| policy_certs.get(&id))
                        .is_some() =>
            {
                let policy_id = listener.tls_policy_id.unwrap();
                let paths = &policy_certs[&policy_id];
                (fs::read(&paths.cert_path)?, fs::read(&paths.key_path)?)
            }
            _ => (default_cert.clone(), default_key.clone()),
        };

        let cert_path = certs_dir.join(format!("https-{}.pem", port));
        let key_path = certs_dir.join(format!("https-{}.key", port));
        fs::write(&cert_path, cert_bytes)?;
        fs::write(&key_path, key_bytes)?;
        map.insert(
            port,
            CertPaths {
                cert_path,
                key_path,
            },
        );
    }

    Ok(map)
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
