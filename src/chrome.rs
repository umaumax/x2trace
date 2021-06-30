use std::collections::HashMap;
use std::time::Duration;

use serde::Serializer;

fn as_micros<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    let v = (d.as_secs() * 1_000_000) + (d.subsec_nanos() as u64 / 1_000);
    s.serialize_u64(v)
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Debug)]
pub enum EventType {
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
impl Default for EventType {
    fn default() -> Self {
        EventType::DurationBegin
    }
}

#[derive(Serialize, Clone, Default, Debug)]
pub struct Event {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "cat")]
    pub category: String,
    #[serde(rename = "ph")]
    pub event_type: EventType,
    #[serde(rename = "ts", serialize_with = "as_micros")]
    #[serde()]
    pub timestamp: Duration,
    #[serde(rename = "dur", serialize_with = "as_micros")]
    pub duration: Duration,
    #[serde(rename = "pid")]
    pub process_id: u32,
    #[serde(rename = "tid")]
    pub thread_id: u32,
    #[serde(rename = "args")]
    pub args: Option<HashMap<String, String>>,
}
