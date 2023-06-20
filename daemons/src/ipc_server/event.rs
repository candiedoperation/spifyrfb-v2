/*
    SpifyRFB - Modern RFB Server implementation using Rust
    Copyright (C) 2023  Atheesh Thirumalairajan

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

static EVENT_LISTENERS: Lazy<RwLock<HashMap<u8, Vec<fn(String)>>>>
    = Lazy::new(|| { RwLock::new(HashMap::new()) });

pub async fn register(event: u8, callback: fn(String)) {
    let mut eventlistener_lock = EVENT_LISTENERS.write().await;
    if eventlistener_lock.get(&event).is_none() {
        eventlistener_lock.insert(event, vec![]);
    }

    /* Insert Callback */
    let event_vector = eventlistener_lock.get(&event).unwrap();
    let mut updated_event = event_vector.clone();
    updated_event.push(callback);

    /* Update EVENT_LISTENERS */
    eventlistener_lock.insert(event, updated_event);
}

pub async fn fire(event: u8, data: String) {
    let eventlistener_lock = EVENT_LISTENERS.read().await;
    let listeners = eventlistener_lock.get(&event);
    if listeners.is_some() {
        let listeners = listeners.unwrap();
        for listener in listeners {
            listener(data.clone());
        }
    }
}