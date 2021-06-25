use std::collections::HashMap;
use std::time::Duration;

#[macro_use]
extern crate serde_derive;
use serde::Serializer;

fn as_micros<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    let v = (d.as_secs() * 1_000_000) + (d.subsec_nanos() as u64 / 1_000);
    s.serialize_u64(v)
}

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
enum EventType {
    #[serde(rename = "B")]
    DurationBegin,
    #[serde(rename = "E")]
    DurationEnd,
    #[serde(rename = "X")]
    Complete,
    #[serde(rename = "I")]
    Instant,
    #[serde(rename = "C")]
    Counter,
    #[serde(rename = "b")]
    AsyncNestableStart,
    #[serde(rename = "n")]
    AsyncNestableInstant,
    #[serde(rename = "e")]
    AsyncNestableEnd,
    #[serde(rename = "s")]
    FlowStart,
    #[serde(rename = "t")]
    FlowStep,
    #[serde(rename = "f")]
    FlowEnd,
    #[serde(rename = "P")]
    Sample,
    #[serde(rename = "N")]
    ObjectCreated,
    #[serde(rename = "O")]
    ObjectSnapshot,
    #[serde(rename = "D")]
    ObjectDestroyed,
    #[serde(rename = "M")]
    Metadata,
    #[serde(rename = "V")]
    MemoryDumpGlobal,
    #[serde(rename = "v")]
    MemoryDumpProcess,
    #[serde(rename = "R")]
    Mark,
    #[serde(rename = "c")]
    ClockSync,
    #[serde(rename = ",")]
    Context,
}

#[derive(Serialize)]
struct Event {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "cat")]
    category: String,
    #[serde(rename = "ph")]
    event_type: EventType,
    #[serde(rename = "ts", serialize_with = "as_micros")]
    #[serde()]
    timestamp: Duration,
    #[serde(rename = "dur", serialize_with = "as_micros")]
    duration: Duration,
    #[serde(rename = "pid")]
    process_id: u32,
    #[serde(rename = "tid")]
    thread_id: u32,
    #[serde(rename = "args")]
    args: Option<HashMap<String, String>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = HashMap::new();
    args.insert(String::from("args"), String::from("value"));
    let event = Event {
        args: Some(args),
        category: String::from("category"),
        duration: Duration::from_millis(1234),
        event_type: EventType::Complete,
        name: String::from("name"),
        process_id: 123,
        thread_id: 123,
        timestamp: Duration::from_millis(1000),
    };
    let json = serde_json::to_string(&event)?;
    println!("{}", json);

    let person = Person {
        name: "nanoha".to_string(),
        age: 10,
    };
    let json = serde_json::to_string(&person)?;
    println!("{}", json);

    Ok(())
}
