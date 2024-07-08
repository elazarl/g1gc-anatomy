use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

/*
{
 "recording": {
  "events": []
 }
}
*/

#[derive(Deserialize, Debug, Clone)]
pub struct JfrMain {
    pub recording: JfrRecording,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JfrRecording {
    pub events: Vec<JfrEvent>,
}
/*
{
  "type": "jdk.TenuringDistribution",
  "values": {
    "startTime": "2024-07-01T09:20:18.758406750+02:00",
    "gcId": 15,
    "age": 14,
    "size": 64
  }
}
*/
#[derive(Deserialize, Debug, Clone)]
pub struct TenuringDistribution {
    #[serde(rename = "startTime", deserialize_with = "deser_ts_ms")]
    pub start_time: DateTime<Utc>,
    #[serde(rename = "gcId")]
    pub gc_id: u64,
    pub age: u64,
    pub size: u64,
}
#[derive(Deserialize, Debug, Clone)]
pub enum GCWhen {
    #[serde(rename = "Before GC")]
    Before,
    #[serde(rename = "After GC")]
    After,
}
/*
"type": "jdk.PromoteObjectOutsidePLAB",
"values": {
  "startTime": "2024-07-01T09:20:16.230667208+02:00",
  "eventThread": {
    "osName": "Gang worker#6 (Parallel GC Threads)",
    "osThreadId": 12035,
    "javaName": null,
    "javaThreadId": 0,
    "group": null
  },
  "gcId": 0,
  "objectClass": {
    "classLoader": {
      "type": null,
      "name": "<bootloader>"
    },
    "name": "[B",
    "package": null,
    "modifiers": 0
  },
  "objectSize": 8606,
  "tenuringAge": 0,
  "tenured": false
}
*/
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PromoteObjectOutsidePLAB {
    #[serde(deserialize_with = "deser_ts_ms")]
    pub start_time: DateTime<Utc>,
    pub gc_id: u64,
    pub object_size: u64,
    pub tenuring_age: u64,
    pub tenured: bool,
}
/*
"type": "jdk.PromoteObjectInNewPLAB",
"values": {
  "startTime": "2024-07-01T09:20:16.230676500+02:00",
  "eventThread": {
    "osName": "Gang worker#4 (Parallel GC Threads)",
    "osThreadId": 11523,
    "javaName": null,
    "javaThreadId": 0,
    "group": null
  },
  "gcId": 0,
  "objectClass": {
    "classLoader": {
      "type": null,
      "name": "<bootloader>"
    },
    "name": "java\/lang\/ref\/Reference$ReferenceHandler",
    "package": {
      "name": "java\/lang\/ref",
      "exported": true
    },
    "modifiers": 32
  },
  "objectSize": 47,
  "tenuringAge": 0,
  "tenured": false,
  "plabSize": 4096
}
*/
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PromoteObjectInNewPLAB {
    #[serde(deserialize_with = "deser_ts_ms")]
    pub start_time: DateTime<Utc>,
    pub gc_id: u64,
    pub object_size: u64,
    pub tenuring_age: u64,
    pub tenured: bool,
    pub plab_size: u64,
}
/*
"type": "jdk.GCHeapSummary",
"values": {
  "startTime": "2024-07-03T13:12:26.119271605+02:00",
  "gcId": 0,
  "when": "Before GC",
  "heapSpace": {
    "start": 33176944640,
    "committedEnd": 33285996544,
    "committedSize": 109051904,
    "reservedEnd": 33285996544,
    "reservedSize": 109051904
  },
  "heapUsed": 25165824
}
*/
#[derive(Deserialize, Debug, Clone)]
pub struct GCHeapSummary {
    #[serde(rename = "startTime", deserialize_with = "deser_ts_ms")]
    pub start_time: DateTime<Utc>,
    pub when: GCWhen,
    #[serde(rename = "gcId")]
    pub gc_id: u64,
    #[serde(rename = "heapUsed")]
    pub heap_used: u64,
}
/*
"startTime": "2024-07-01T09:20:16.230469750+02:00",
"gcId": 0,
"when": "Before GC",
"edenUsedSize": 25165824,
"edenTotalSize": 25165824,
"survivorUsedSize": 0,
"numberOfRegions": 104
*/
#[derive(Deserialize, Debug, Clone)]
pub struct G1HeapSummary {
    #[serde(rename = "startTime", deserialize_with = "deser_ts_ms")]
    pub start_time: DateTime<Utc>,
    pub when: GCWhen,
    #[serde(rename = "gcId")]
    pub gc_id: u64,
    #[serde(rename = "edenUsedSize")]
    pub eden_used: u64,
    #[serde(rename = "edenTotalSize")]
    pub eden_total: u64,
    #[serde(rename = "survivorUsedSize")]
    pub survivor_used: u64,
}
/*
"type": "jdk.G1GarbageCollection",
"values": {
  "startTime": "2024-07-05T13:49:17.514902791+03:00",
  "duration": "PT0.002919334S",
  "gcId": 2,
  "type": "Normal"
}
*/
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum CollectionType {
    Normal,
    #[serde(rename = "Prepare Mixed")]
    PrepareMixed,
    #[serde(rename = "Concurrent Start")]
    ConcurrentStart,
    Mixed,
    #[serde(other)]
    #[default]
    Unknown,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct G1GarbageCollection {
    #[serde(deserialize_with = "deser_ts_ms")]
    pub start_time: DateTime<Utc>,
    pub gc_id: u64,
    #[serde(rename = "type")]
    pub type_: CollectionType,
}
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum JfrEvent {
    #[serde(rename = "jdk.TenuringDistribution")]
    TenuringDistribution { values: TenuringDistribution },
    #[serde(rename = "jdk.GCHeapSummary")]
    GCHeapSummary { values: GCHeapSummary },
    #[serde(rename = "jdk.G1HeapSummary")]
    G1HeapSummary { values: G1HeapSummary },
    #[serde(rename = "jdk.G1GarbageCollection")]
    G1GarbageCollection { values: G1GarbageCollection },
    #[serde(rename = "jdk.PromoteObjectOutsidePLAB")]
    PromoteObjectOutsidePLAB { values: PromoteObjectOutsidePLAB },
    #[serde(rename = "jdk.PromoteObjectInNewPLAB")]
    PromoteObjectInNewPLAB { values: PromoteObjectInNewPLAB },
    #[serde(other)]
    Unkown,
}

impl JfrEvent {
    pub fn gc_id(&self) -> Option<u64> {
        match &self {
            JfrEvent::TenuringDistribution { values } => Some(values.gc_id),
            JfrEvent::GCHeapSummary { values } => Some(values.gc_id),
            JfrEvent::G1HeapSummary { values } => Some(values.gc_id),
            JfrEvent::PromoteObjectOutsidePLAB { values } => Some(values.gc_id),
            JfrEvent::PromoteObjectInNewPLAB { values } => Some(values.gc_id),
            JfrEvent::G1GarbageCollection { values } => Some(values.gc_id),
            JfrEvent::Unkown => None,
        }
    }
}

fn deser_ts_ms<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp = String::deserialize(deserializer)?; // Deserialize as i64
    DateTime::parse_from_rfc3339(timestamp.as_str())
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| serde::de::Error::custom(e.to_string()))
}

#[allow(unused)]
fn deser_dur_ms<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let ms = u64::deserialize(deserializer)?; // Deserialize as i64
    Ok(Duration::from_millis(ms))
}
