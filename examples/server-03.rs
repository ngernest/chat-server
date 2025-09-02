use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};

// Echo server that adds a heart emoji to the end of every message

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    
    loop {
        let (mut tcp, _) = server.accept().await?;
        let mut buffer = [0u8; 16];
        
        loop {
            let n = tcp.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            
            // Convert byte slice to a `String`
            let mut line = String::from_utf8(buffer[..n].to_vec())?;
            
            // Remove the line-terminating chars that were added by Telnet
            line.pop(); // remove \n char
            line.pop(); // remove \r char

            // Add our own line terminator
            line.push_str(" ❤️\n");
            
            let _ = tcp.write(line.as_bytes()).await?;
        }
    }
}
