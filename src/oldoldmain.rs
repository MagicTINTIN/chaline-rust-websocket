//use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::accept_async;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::{ServerConfig, Certificate, PrivateKey};
use std::{sync::Arc, fs::File, io::BufReader, net::SocketAddr};
use futures::{StreamExt, SinkExt}; // Needed for split
//use tokio_tungstenite::tungstenite::protocol::Message; // Keep if you're working with WebSocket messages

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load certificate and private key
    let cert_file = &mut BufReader::new(File::open("/etc/ssl/cloudflare/origin.crt")?);
    let key_file = &mut BufReader::new(File::open("/etc/ssl/cloudflare/origin.key")?);
    let certs = rustls_pemfile::certs(cert_file)?.into_iter().map(Certificate).collect();
    let mut keys = rustls_pemfile::rsa_private_keys(key_file)?;
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, PrivateKey(keys.remove(0)))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let addr = SocketAddr::from(([0, 0, 0, 0], 443));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("WebSocket server listening on wss://{}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let tls_stream = acceptor.accept(stream).await.unwrap();
            let ws_stream = accept_async(tls_stream).await.unwrap();
            println!("New WebSocket connection");

            let (mut write, mut read) = ws_stream.split();
            while let Some(msg) = read.next().await {
                if let Ok(msg) = msg {
                    if msg.is_text() || msg.is_binary() {
                        write.send(msg).await.unwrap();
                    }
                }
            }
        });
    }

    Ok(())
}

