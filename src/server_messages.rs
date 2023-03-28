use log::info;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures_channel::mpsc::UnboundedSender;
use tungstenite::protocol::Message;

use crate::models::{communication::Response, lobby::User};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;

pub fn send_message(response: Response, peer_map: PeerMap, id: &String) {
    info!("Sending msg to: {}", &id);

    let peers = peer_map.lock().unwrap();
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| peer_addr == &id)
        .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }

    info!("Message sent successfully to: {}", &id);
}

pub fn broadcast_message_all(response: Response, peer_map: &PeerMap) {
    info!("Sending broadcast to all connections");
    let peers = peer_map.lock().unwrap();
    let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }
    info!("Broadcast sent successfully to all connections");
}

pub fn broadcast_message_except(response: Response, peer_map: PeerMap, addr: &SocketAddr) {
    info!("Sending broadcast to all connections except: {}", &addr);
    // let peers = peer_map.lock().unwrap();
    // let broadcast_recipients = peers
    //     .iter()
    //     .filter(|(peer_addr, _)| peer_addr != &addr)
    //     .map(|(_, ws_sink)| ws_sink);

    // for recp in broadcast_recipients {
    //     recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
    //         .unwrap();
    // }
    info!(
        "Broadcast sent successfully to all connections except: {}",
        &addr
    );
}

pub fn broadcast_message_room_all(response: Response, peer_map: PeerMap, user_list: &Vec<User>) {
    info!("Sending broadcast to all room players");

    let peers = peer_map.lock().unwrap();
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| {
            user_list
                .iter()
                .map(|user| &user.id)
                .any(|id| &id == peer_addr)
        })
        .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }

    info!("Broadcast sent successfully to all room players");
}

pub fn broadcast_message_room_except(
    response: Response,
    peer_map: PeerMap,
    user_list: &Vec<User>,
    id: &String,
) {
    info!("Sending broadcast to all room players except: {}", &id);

    let peers = peer_map.lock().unwrap();
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| {
            user_list
                .iter()
                .map(|user| &user.id)
                .any(|user_id| &user_id == peer_addr)
        } && peer_addr != &id)
        .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Text(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }

    info!(
        "Broadcast sent successfully to all room players except: {}",
        &id
    );
}
