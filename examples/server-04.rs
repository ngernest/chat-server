use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    
    loop {
        // Accept the TCP connection (`tcp` is a `TcpStream`)
        let (mut tcp, _) = server.accept().await?;
        
        // Split the `TcpStream` into a `ReadHalf` & a `WriteHalf`
        let (reader, writer) = tcp.split();
        
        // `LinesCodec` handles low-level details of converting a byte stream
        // into a stream of UTF-8 strings delimited by newlines.
        // Note: a `Stream` is the async version of an `Iterator`.
        let mut stream = FramedRead::new(reader, LinesCodec::new());
        
        // A `Sink` consumes values instead of producing values
        let mut sink = FramedWrite::new(writer, LinesCodec::new());
        
        // Get a message from a stream, add a heart to it, 
        // then send it to the sink
        while let Some(Ok(mut msg)) = stream.next().await {
            msg.push_str(" ❤️");
            sink.send(msg).await?;
        }
    }
}
