use std::fs;
use std::net::{IpAddr, Ipv4Addr, TcpStream};
use std::sync::Arc;

use rustls::ClientConnection;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig, ServerConnection, StreamOwned};

use crate::tls_error::TlsError;

pub fn client_tls_stream(
    stream_to_encrip: TcpStream,
    server_adder: &str,
) -> Result<StreamOwned<ClientConnection, TcpStream>, TlsError> {
    // Levanto el archivo cert.pem para los certificados
    println!("\n\n[DEBUG TLS] File Read");
    let cert_file = fs::read("certs/cert.der").map_err(|_| TlsError::PemfileError)?;
    println!("\n\n[DEBUG TLS] File Read");

    let certs = vec![CertificateDer::from(cert_file)];

    // Levanto el archivo key.pem para las llaves
    let key_file = fs::read("certs/key.der").map_err(|_| TlsError::PemfileError)?;
    let _key = rustls::pki_types::PrivateKeyDer::Pkcs8(
        rustls::pki_types::PrivatePkcs8KeyDer::from(key_file),
    );

    let mut root_store = RootCertStore::empty();
    let (_added, _ignored) = root_store.add_parsable_certificates(certs);

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    println!("\n\n[DEBUG TLS] Config Success");

    let server_adder = String::from(server_adder);

    println!("\n\n[DEBUG TLS] Server Adder: {:?}", &server_adder);

    let ipv4 = IpAddr::V4(Ipv4Addr::LOCALHOST);

    println!("\n\n[DEBUG TLS] IPV4: {:?}", &ipv4);

    let server_name = ServerName::from(ipv4);
    let connection = match ClientConnection::new(Arc::new(config), server_name) {
        Ok(connect) => connect,
        Err(_) => return Err(TlsError::ClientConnectionError),
    };

    let tls_stream = StreamOwned::new(connection, stream_to_encrip);
    println!("\n\n[DEBUG TLS] Handshake Finishing");
    Ok(tls_stream)
}

pub fn server_tls_config() -> Result<Arc<ServerConfig>, TlsError> {
    // Levanto el archivo cert.pem para los certificados
    let cert_file = fs::read("certs/cert.der").map_err(|_| TlsError::PemfileError)?;

    let certs = vec![CertificateDer::from(cert_file)];

    let key_file = fs::read("certs/key.der").map_err(|_| TlsError::PemfileError)?;

    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
        key_file,
    ));

    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|_| TlsError::ServerConfigError)?;

    Ok(Arc::new(server_config))
}

pub fn server_try_tls_handshake(
    config: Arc<ServerConfig>,
    stream: TcpStream,
) -> Result<StreamOwned<ServerConnection, TcpStream>, TlsError> {
    let connection = ServerConnection::new(config).map_err(|_| TlsError::HandshakeError)?;
    let tls = StreamOwned::new(connection, stream);

    Ok(tls)
}
