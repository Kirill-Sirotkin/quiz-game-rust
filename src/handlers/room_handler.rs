use futures_timer::Delay;
use log::info;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{helpers::get_list_element, models::lobby::Room};

pub async fn handle_room_timeout(room_id: String, room_list: Arc<Mutex<Vec<Room>>>) {
    Delay::new(Duration::from_secs(10)).await;

    let room_info = get_list_element(&room_id, room_list.clone()).unwrap();
    if room_info.current_players <= 0 {
        info!("Removing room: {}", &room_id);

        let index = room_list
            .lock()
            .unwrap()
            .iter()
            .position(|room| room.id == room_id)
            .unwrap();
        room_list.lock().unwrap().remove(index);
    }
}
