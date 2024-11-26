use futures::{StreamExt, SinkExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::{ServerConfig, Certificate, PrivateKey};
use tokio_tungstenite::accept_async;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cert_file = &mut BufReader::new(File::open("/etc/ssl/private/mtc")?);
    let key_file = &mut BufReader::new(File::open("/etc/ssl/private/mtk")?);
    let cert_chain = certs(cert_file)?
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys = pkcs8_private_keys(key_file)?;
    if keys.is_empty() {
        return Err("No private key found".into());
    }

    // TLS server
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, PrivateKey(keys.remove(0)))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    // start TCP listener
    let listener = TcpListener::bind("[::]:8443").await?;
    println!("Listening on wss://[::]:8443");

    while let Ok((stream, _)) = listener.accept().await {
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            // accept TLS connection
            let tls_stream = match acceptor.accept(stream).await {
                Ok(tls_stream) => tls_stream,
                Err(err) => {
                    eprintln!("TLS handshake failed: {}", err);
                    return;
                }
            };

            // upgrade to a WebSocket connection
            let ws_stream = accept_async(tls_stream).await.unwrap();
            println!("New WebSocket connection established");

            // handle incoming messages (echo them back)
            let (mut write, mut read) = ws_stream.split();
            while let Some(Ok(msg)) = read.next().await {
                if let tokio_tungstenite::tungstenite::protocol::Message::Text(txt) = msg {
                    println!("Received: {}", txt);
                    if txt.contains("new micasend message") {
                        println!("ping sent");
                        write
                            .send(tokio_tungstenite::tungstenite::protocol::Message::Text("new message notification".to_string())) //txt
                            .await
                            .unwrap();
                    }
                }
            }
            println!("socket connection ended");
        });
    }

    Ok(())
}

