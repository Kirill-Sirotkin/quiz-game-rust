use crate::{
    helpers::{edit_list_element, get_list_element, get_room_user_list},
    models::{
        communication::Response,
        lobby::{Room, User},
    },
    server_messages::broadcast_message_room_all,
};
use core::time;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_delay_queue::delay_queue;
use futures_timer::Delay;
use futures_util::{
    future::{self, ok, Pending},
    pin_mut, poll, FutureExt, StreamExt,
};
use log::info;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    task::Context,
    time::Duration,
};
use tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<String, Tx>>>;
type UserList = Arc<Mutex<Vec<User>>>;
type RoomList = Arc<Mutex<Vec<Room>>>;
type Lists = (PeerMap, UserList, RoomList);

pub async fn handle_room_timeout(room_id: String, room_list: RoomList) {
    Delay::new(Duration::from_secs(10)).await;

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
    room_id: String,
    lists: Lists,
    rx: UnboundedReceiver<bool>,
) {
    // let timer = Delay::new(Duration::from_secs(10));
    let timer = Delay::new(Duration::from_secs(10));

    // let (delay_queue, rxx) = delay_queue::<String>();
    // let delay_handle = delay_queue.insert("COMPLETE!!!!".to_string(), Duration::from_secs(10));
    // let timer_recv = rxx.receive();

    // let receive_future = rx.for_each(|msg| {
    //     match msg {
    //         true => {
    //             println!("Received future reset!");
    //         }
    //         false => (),
    //     }
    //     future::ready(())
    // });

    let receive_future = rx
        .take_while(|msg| future::ready(msg.clone()))
        .into_future();

    // loop {
    //     let msg = rx.next().await;

    //     match msg {
    //         Some(msg) => {
    //             println!("Received timeout message: {}", msg);
    //             return;
    //         }
    //         None => {
    //             println!("Connection stopped")
    //         }
    //     }
    // }

    // pin_mut!(timer_recv, receive_future);
    // future::select(timer_recv, receive_future).await;
    // println!("End of timeout thread");

    // println!("WOW! queue worked!: {:?}", rxx.receive().await);
    // println!("End of timeout thread");

    // IF USER DISCONNECTS AS TIMEOUTS ARE CHECKING PEERMAP TO CONTAIN KEY,
    // THEN THE KEY WILL NOT EXIST, EVEN THOUGH TECHNICALLY USER DOES EXISTS.
    // THIS LEADS TO PREMATURE REMOVAL AT THE TIME, WHEN USER IS RECONNECTING.
    // THUS THE USER GETS BUGGED

    // TRY FIX BY EXTENDING THIS THREAD'S TIMER?

    pin_mut!(timer, receive_future);
    let select = future::select(timer, receive_future).await;
    match select {
        future::Either::Left(_) => {
            println!("Timer finished first!");

            if !lists.0.lock().unwrap().contains_key(&user_id) {
                println!("Removing user: {}", &user_id);
                info!("Removing user: {}", &user_id);

                let index = lists
                    .1
                    .lock()
                    .unwrap()
                    .iter()
                    .position(|user| user.id == user_id);

                match index {
                    Some(index) => {
                        lists.1.lock().unwrap().remove(index);
                        edit_list_element(&room_id, lists.2.clone(), |room| {
                            room.current_players -= 1;
                        })
                        .unwrap();

                        let user_list = get_room_user_list(&room_id, lists.1.clone());
                        let update_user_list_response = Response::updateUserList {
                            userList: user_list.clone(),
                        };
                        broadcast_message_room_all(
                            update_user_list_response,
                            lists.0.clone(),
                            &user_list,
                        );

                        let room_info = get_list_element(&room_id, lists.2.clone()).unwrap();
                        if room_info.current_players <= 0 {
                            println!("Removing room: {}", &room_id);
                            info!("Removing room: {}", &room_id);

                            let index = lists
                                .2
                                .lock()
                                .unwrap()
                                .iter()
                                .position(|room| room.id == room_id);
                            match index {
                                Some(index) => {
                                    lists.2.lock().unwrap().remove(index);
                                }
                                None => println!("No index found for room!"),
                            }
                        }
                    }
                    None => println!("No index found for user!"),
                };
            } else {
                println!("NOT removing user: {}", &user_id);
            }
        }
        future::Either::Right(_) => {
            println!("Stopping timeout");
        }
    }
}
