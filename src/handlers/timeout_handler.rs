use crate::{
    helpers::get_list_element,
    models::lobby::{Room, User},
};
use futures_channel::mpsc::UnboundedSender;
use futures_timer::Delay;
use log::info;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;

pub async fn handle_room_timeout(room_id: String, room_list: RoomList) {
    Delay::new(Duration::from_secs(2)).await;

    let room_info = get_list_element(&room_id, room_list.clone()).unwrap();
    if room_info.current_players <= 0 {
        println!("Removing room: {}", &room_id);
        info!("Removing room: {}", &room_id);

        let index = room_list
            .lock()
            .unwrap()
            .iter()
            .position(|room| room.id == room_id);
        match index {
            Some(index) => {
                room_list.lock().unwrap().remove(index);
            }
            None => println!("No index found for room!"),
        }
    } else {
        println!("NOT removing room: {}", &room_id);
    }
}

pub async fn handle_user_timeout(user_id: String, user_list: UserList, peer_map: PeerMap) {
    Delay::new(Duration::from_secs(2)).await;

    if !peer_map.lock().unwrap().contains_key(&user_id) {
        println!("Removing user: {}", &user_id);
        info!("Removing user: {}", &user_id);

        let index = user_list
            .lock()
            .unwrap()
            .iter()
            .position(|user| user.id == user_id);

        match index {
            Some(index) => {
                user_list.lock().unwrap().remove(index);
            }
            None => println!("No index found for user!"),
        };
    } else {
        println!("NOT removing user: {}", &user_id);
    }
}
