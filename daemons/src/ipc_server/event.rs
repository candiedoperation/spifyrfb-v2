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