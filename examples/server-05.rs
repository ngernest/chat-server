use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

// Include the contents of `help-01.txt` as a string
const HELP_MSG: &str = include_str!("shared/help-01.txt");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    loop {
        let (mut tcp, _) = server.accept().await?;
        let (reader, writer) = tcp.split();

        let mut stream = FramedRead::new(reader, LinesCodec::new());
        let mut sink = FramedWrite::new(writer, LinesCodec::new());
        
        // Send a list of server commands to the sink 
        sink.send(HELP_MSG).await?;
        
        while let Some(Ok(mut msg)) = stream.next().await {
            // Handle /help command
            if msg.starts_with("/help") {
                sink.send(HELP_MSG).await?;
            } else if msg.starts_with("/quit") {
                // Handle /quit command
                break;
            } else {
                // Handle regular messages
                msg.push_str(" ❤️");
                sink.send(msg).await?;
            }
        }
    }
}
