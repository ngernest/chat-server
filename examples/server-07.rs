use futures::{SinkExt, StreamExt};
use tokio::{net::{TcpListener, TcpStream}, sync::broadcast::{self, Sender}};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

const HELP_MSG: &str = include_str!("shared/help-01.txt");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    // Create broadcast channel for communicating between different clients
    // `tx` is the sender for the channel 
    let (tx, _) = broadcast::channel::<String>(32);
    loop {
        let (tcp, _) = server.accept().await?;
        // For each connected client, we have to clone the broadcast channel
        tokio::spawn(handle_user(tcp, tx.clone()));
    }
}

async fn handle_user(mut tcp: TcpStream, tx: Sender<String>) -> anyhow::Result<()> {
    let (reader, writer) = tcp.split();
    let mut stream = FramedRead::new(reader, LinesCodec::new());
    let mut sink = FramedWrite::new(writer, LinesCodec::new());
    
    // Get a receiver from the sender
    let mut rx = tx.subscribe();
    sink.send(HELP_MSG).await?;
    while let Some(Ok(mut user_msg)) = stream.next().await {
        if user_msg.starts_with("/help") {
            sink.send(HELP_MSG).await?;
        } else if user_msg.starts_with("/quit") {
            break;
        } else {
            user_msg.push_str(" ❤️");
            // Send all messages to the channel
            tx.send(user_msg)?;
        }
         // receive all of our and others'
        // messages from the channel
        let peer_msg = rx.recv().await?;
        sink.send(peer_msg).await?;
    }
    Ok(())
}
