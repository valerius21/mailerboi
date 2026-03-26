#[cfg(test)]
mod tests {
    use async_imap::Client;
    use async_native_tls::TlsConnector;
    use tokio::net::TcpStream;

    #[tokio::test]
    #[ignore]
    async fn spike_imap_tls_connection() {
        let tcp = TcpStream::connect("127.0.0.1:3993").await.unwrap();
        let tls_connector = TlsConnector::new()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true);
        let tls_stream = tls_connector.connect("localhost", tcp).await.unwrap();

        let client = Client::new(tls_stream);
        let mut session = client
            .login("test@localhost", "test")
            .await
            .map_err(|(err, _)| err)
            .unwrap();

        session.select("INBOX").await.unwrap();
        session.logout().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn spike_imap_plain_connection() {
        let tcp = TcpStream::connect("127.0.0.1:3143").await.unwrap();
        let client = Client::new(tcp);
        let mut session = client
            .login("test2@localhost", "test2")
            .await
            .map_err(|(err, _)| err)
            .unwrap();

        session.select("INBOX").await.unwrap();
        session.logout().await.unwrap();
    }
}
