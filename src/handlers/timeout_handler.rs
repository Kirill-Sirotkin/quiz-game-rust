use crate::{
    helpers::get_list_element,
    models::lobby::{Room, User},
};
use core::time;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_timer::Delay;
use futures_util::{
    future::{self, ok, Pending},
    pin_mut, StreamExt,
};
use log::info;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    time::Duration,
};
use tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;

pub async fn handle_room_timeout(room_id: String, room_list: RoomList) {
    Delay::new(Duration::from_secs(5)).await;

    let room_info = match get_list_element(&room_id, room_list.clone()) {
        Some(info) => info,
        None => {
            println!("No room info found!");
            return;
        }
    };
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

pub async fn handle_user_timeout(
    user_id: String,
    user_list: UserList,
    peer_map: PeerMap,
    mut rx: UnboundedReceiver<bool>,
) {
    let timer = Delay::new(Duration::from_secs(10));

    // let receive_future = rx.for_each(|msg| {
    //     match msg {
    //         true => {
    //             println!("Received future reset!");
    //             timer.reset(Duration::from_secs(10));
    //             tokio::spawn(future)
    //         }
    //         false => (),
    //     }
    //     future::ready(())
    // });

    // while true {
    //     let msg = rx.next().await;

    //     match msg {
    //         Some(msg) => {
    //             println!("Received timeout message: {}", msg)
    //         }
    //         None => {
    //             println!("Connection stopped")
    //         }
    //     }
    // }

    // IF USER DISCONNECTS AS TIMEOUTS ARE CHECKING PEERMAP TO CONTAIN KEY,
    // THEN THE KEY WILL NOT EXIST, EVEN THOUGH TECHNICALLY USER DOES EXISTS.
    // THIS LEADS TO PREMATURE REMOVAL AT THE TIME, WHEN USER IS RECONNECTING.
    // THUS THE USER GETS BUGGED

    // TRY FIX BY EXTENDING THIS THREAD'S TIMER?

    // pin_mut!(timer, receive_future);
    // future::select(timer, receive_future).await;

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
