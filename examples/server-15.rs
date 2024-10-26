use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex, RwLock}};
use futures::{SinkExt, StreamExt};
use tokio::{net::{TcpListener, TcpStream}, sync::broadcast::{self, Sender}};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

#[path ="shared/lib.rs"]
mod lib;
use lib::{b, random_name};

const HELP_MSG: &str = include_str!("shared/help-04.txt");

#[derive(Clone)]
struct Names(Arc<Mutex<HashSet<String>>>);

impl Names {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }
    fn insert(&self, name: String) -> bool {
        self.0.lock().unwrap().insert(name)
    }
    fn remove(&self, name: &str) -> bool {
        self.0.lock().unwrap().remove(name)
    }
    fn get_unique(&self) -> String {
        let mut name = random_name();
        let mut guard = self.0.lock().unwrap();
        while !guard.insert(name.clone()) {
            name = random_name();
        }
        name
    }
}

struct Room {
    tx: Sender<String>,
}

impl Room {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(32);
        Self {
            tx,
        }
    }
}

const MAIN: &str = "main";

#[derive(Clone)]
struct Rooms(Arc<RwLock<HashMap<String, Room>>>);

impl Rooms {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
    fn join(&self, room_name: &str) -> Sender<String> {
        let read_guard = self.0.read().unwrap();
        if let Some(room) = read_guard.get(room_name) {
            return room.tx.clone();
        }
        drop(read_guard);
        let mut write_guard = self.0.write().unwrap();
        let room = write_guard.entry(room_name.to_owned()).or_insert(Room::new());
        room.tx.clone()
    }
    fn leave(&self, room_name: &str) {
        let read_guard = self.0.read().unwrap();
        let mut delete_room = false;
        if let Some(room) = read_guard.get(room_name) {
            delete_room = room.tx.receiver_count() <= 1;
        }
        drop(read_guard);
        if delete_room {
            let mut write_guard = self.0.write().unwrap();
            write_guard.remove(room_name);
        }
    }
    fn change(&self, prev_room: &str, next_room: &str) -> Sender<String> {
        self.leave(prev_room);
        self.join(next_room)
    }
    fn list(&self) -> Vec<(String, usize)> {
        let mut list: Vec<_> = self
            .0
            .read()
            .unwrap()
            .iter()
            .map(|(name, room)| (name.to_owned(), room.tx.receiver_count()))
            .collect();
        list.sort_by(|a, b| {
            use std::cmp::Ordering::*;
            match b.1.cmp(&a.1) {
                Equal => a.0.cmp(&b.0),
                ordering => ordering,
            }
        });
        list
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    let names = Names::new();
    let rooms = Rooms::new();
    loop {
        let (tcp, _) = server.accept().await?;
        tokio::spawn(handle_user(tcp, names.clone(), rooms.clone()));
    }
}

async fn handle_user(
    mut tcp: TcpStream,
    names: Names,
    rooms: Rooms,
) -> anyhow::Result<()> {
    let (reader, writer) = tcp.split();
    let mut stream = FramedRead::new(reader, LinesCodec::new());
    let mut sink = FramedWrite::new(writer, LinesCodec::new());
    let mut name = names.get_unique();
    sink.send(format!("{HELP_MSG}\nYou are {name}")).await?;
    let mut room_name = MAIN.to_owned();
    let mut room_tx = rooms.join(&room_name);
    let mut room_rx = room_tx.subscribe();
    let _ = room_tx.send(format!("{name} joined {room_name}"));
    let result: anyhow::Result<()> = loop {
        tokio::select! {
            user_msg = stream.next() => {
                let user_msg = match user_msg {
                    Some(msg) => b!(msg),
                    None => break Ok(()),
                };
                if user_msg.starts_with("/help") {
                    b!(sink.send(HELP_MSG).await);
                } else if user_msg.starts_with("/name") {
                    let new_name = user_msg
                        .split_ascii_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_owned();
                    let changed_name = names.insert(new_name.clone());
                    if changed_name {
                        b!(room_tx.send(format!("{name} is now {new_name}")));
                        names.remove(&name);
                        name = new_name;
                    } else {
                        b!(sink.send(format!("{new_name} is already taken")).await);
                    }
                } else if user_msg.starts_with("/join") {
                    let new_room = user_msg
                        .split_ascii_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_owned();
                    if new_room == room_name {
                        b!(sink.send(format!("You are in {room_name}")).await);
                        continue;
                    }
                    b!(room_tx.send(format!("{name} left {room_name}")));
                    room_tx = rooms.change(&room_name, &new_room);
                    room_rx = room_tx.subscribe();
                    room_name = new_room;
                    b!(room_tx.send(format!("{name} joined {room_name}")));
                } else if user_msg.starts_with("/rooms") {
                    let rooms_list = rooms.list();
                    let rooms_list = rooms_list
                        .into_iter()
                        .map(|(name, count)| format!("{name} ({count})"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    b!(sink.send(format!("Rooms - {rooms_list}")).await);
                } else if user_msg.starts_with("/quit") {
                    break Ok(());
                } else {
                    b!(room_tx.send(format!("{name}: {user_msg}")));
                }
            },
            peer_msg = room_rx.recv() => {
                let peer_msg = b!(peer_msg);
                b!(sink.send(peer_msg).await);
            },
        }
    };
    let _ = room_tx.send(format!("{name} left {room_name}"));
    rooms.leave(&room_name);
    names.remove(&name);
    result
}
