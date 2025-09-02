use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Bind a TCP listener to port `42069` using `tokio::net::TcpListener` 
    // (which is async, unlike `std::net::TcpListener`).
    // This returns a `Future` which we must `await` in order for anything to hapen 
    // (since futures are lazy in Rust).
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    
    // Accept a connection, create a buffer
    let (mut tcp, _) = server.accept().await?;
    let mut buffer = [0u8; 16];
    
    // Loop until the connection closes
    loop {
        // Read bytes from the connection into the buffer
        let n = tcp.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        // Write those bytes back into the buffer
        let _ = tcp.write(&buffer[..n]).await?;
    }
    Ok(())
}
