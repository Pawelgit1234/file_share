use std::{fs, fs::File, io::BufReader, path::Path, sync::Arc};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    ServerConfig,
};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::TlsAcceptor;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

pub fn create_or_load_tls(cert_path: &str, key_path: &str) -> anyhow::Result<TlsAcceptor> {
    let config = if Path::new(cert_path).exists() && Path::new(key_path).exists() {
        println!("Using existing TLS certificate and key");
        load_tls_config(cert_path, key_path)?
    } else {
        println!("Generating self-signed TLS certificate...");
        generate_self_signed_tls(cert_path, key_path)?
    };

    Ok(TlsAcceptor::from(Arc::new(config)))
}

fn load_tls_config(cert_path: &str, key_path: &str) -> anyhow::Result<ServerConfig> {
    // read certificate
    let mut cert_reader = BufReader::new(File::open(cert_path)?);
    let cert_chain = certs(&mut cert_reader)
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(CertificateDer::from)
        .collect::<Vec<_>>();

    // read key
    let mut key_reader = BufReader::new(File::open(key_path)?);
    let mut keys = pkcs8_private_keys(&mut key_reader)
        .collect::<std::io::Result<Vec<_>>>()?;

    if keys.is_empty() {
        anyhow::bail!("No private key found in {key_path}");
    }

    let key = PrivateKeyDer::from(keys.remove(0));

    Ok(ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)?)
}

fn generate_self_signed_tls(cert_path: &str, key_path: &str) -> anyhow::Result<ServerConfig> {
    let mut params = CertificateParams::new(vec![
        "localhost".to_string(),
        "127.0.0.1".to_string()
    ])?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "FileShare Server");
    params.distinguished_name = dn;

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    if let Some(parent) = Path::new(cert_path).parent() {
        fs::create_dir_all(parent)?;
    }

    // save PEM files
    fs::create_dir_all(Path::new(cert_path).parent().unwrap())?;
    fs::write(cert_path, cert.pem())?;
    fs::write(key_path, key_pair.serialize_pem())?;
    println!("Self-signed certificate generated at {cert_path}");

    // convert for rustls
    let cert_der = CertificateDer::from(cert.der().clone());
    let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_pair.serialize_der().clone()));

    Ok(ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?)
}
