use std::{collections::HashSet, sync::{Arc, Mutex}};
use futures::{SinkExt, StreamExt};
use tokio::{net::{TcpListener, TcpStream}, sync::broadcast::{self, Sender}};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

#[path ="shared/lib.rs"]
mod lib;
use lib::random_name;

const HELP_MSG: &str = include_str!("shared/help-02.txt");

#[derive(Clone)]
struct Names(Arc<Mutex<HashSet<String>>>);

impl Names {
    /// We have to wrap a HashSet inside `Arc<Mutex<...>>` 
    /// to share mutable state between threads
    fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }
    
    /// Returns true if name was inserted, i.e. the name is unique
    fn insert(&self, name: String) -> bool {
        self.0.lock().unwrap().insert(name)
    }

    /// Removes an element from the HashSet
    fn remove(&self, name: &String) -> bool {
        self.0.lock().unwrap().remove(name)
    }

    /// Returns unique name
    fn get_unique(&self) -> String {
        let mut name = random_name();
        let mut guard = self.0.lock().unwrap();
        while !guard.insert(name.clone()) {
            name = random_name();
        }
        name
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    let (tx, _) = broadcast::channel::<String>(32);
    let names = Names::new();
    loop {
        let (tcp, _) = server.accept().await?;
        tokio::spawn(handle_user(tcp, tx.clone(), names.clone()));
    }
}

async fn handle_user(
    mut tcp: TcpStream,
    tx: Sender<String>,
    names: Names,
) -> anyhow::Result<()> {
    let (reader, writer) = tcp.split();
    let mut stream = FramedRead::new(reader, LinesCodec::new());
    let mut sink = FramedWrite::new(writer, LinesCodec::new());
    let mut rx = tx.subscribe();
    let mut name = names.get_unique();
    sink.send(HELP_MSG).await?;
    sink.send(format!("You are {name}")).await?;
    loop {
        tokio::select! {
            user_msg = stream.next() => {
                let user_msg = match user_msg {
                    Some(msg) => msg?,
                    None => break,
                };
                if user_msg.starts_with("/help") {
                    sink.send(HELP_MSG).await?;
                } else if user_msg.starts_with("/name") {
                    let new_name = user_msg
                        .split_ascii_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_owned();
                    let changed_name = names.insert(new_name.clone());
                    if changed_name {
                        tx.send(format!("{name} is now {new_name}"))?;
                        names.remove(&name);
                        name = new_name;
                    } else {
                        sink.send(format!("{new_name} is already taken")).await?;
                    }
                } else if user_msg.starts_with("/quit") {
                    break;
                } else {
                    tx.send(format!("{name}: {user_msg}"))?;
                }
            },
            peer_msg = rx.recv() => {
                sink.send(peer_msg?).await?;
            },
        }
    };
    Ok(())
}
