use std::collections::HashMap;
use std::time::Duration;

use x2trace::chrome;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = HashMap::new();
    args.insert(String::from("args"), String::from("value"));
    let event = chrome::Event {
        args: Some(args),
        category: String::from("category"),
        duration: Duration::from_millis(1234),
        event_type: chrome::EventType::Complete,
        name: String::from("name"),
        process_id: 123,
        thread_id: 123,
        timestamp: Duration::from_millis(1000),
    };
    let json = serde_json::to_string(&event)?;
    println!("{}", json);
    Ok(())
}
