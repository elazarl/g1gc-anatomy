pub mod jfr;
use crate::jfr::{CollectionType, JfrEvent, JfrMain};
use std::{
    collections::{BTreeMap, HashSet},
    io::Write,
    process::Command,
};

use axum::{
    extract::{Query, State},
    http::header,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use clap::Parser;
use itertools::Itertools;
use plotly::{common, layout::Axis, plot::Plot, Bar, Layout, Scatter};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    jfr_file: String,
    #[arg(short, long, default_value = "localhost:3000")]
    addr: String,
    #[arg(short, long, default_value = "false")]
    webbrowser: bool,
}

#[derive(Default, Clone, Debug)]
pub struct Graphs {
    ages: Vec<Scatter<u64, u64>>,
    gcs: Vec<Bar<f64, u64>>,
    gcs_labels: Vec<String>,
    gcs_ticks: Vec<f64>,
}

#[derive(Default, Clone)]
struct Candle {
    gc_id: u64,
    before_gc: u64,
    young_before: u64,
    young_after: u64,
    survivors_before: u64,
    survivors_after: u64,
    after_gc: u64,
    tenured: u64,
    gc_name: String,
    gc_pause_name: String,
    tenuring_threshold: u64,
    collection_type: CollectionType,
}
impl Candle {
    fn title(&self) -> String {
        if let CollectionType::Unknown = self.collection_type {
            self.gc_pause_name.clone()
        } else {
            format!("{:?}", self.collection_type,)
        }
    }
}

impl JfrMain {
    pub fn to_graphs(&self, collection_type_filter: HashSet<CollectionType>) -> Graphs {
        let mut graphs: Graphs = Default::default();
        let mut gc_id_to_candle: BTreeMap<u64, Candle> = BTreeMap::new();
        for evt in &self.recording.events {
            let gc_id = evt.gc_id();
            let candle: &mut Candle = gc_id_to_candle
                .entry(gc_id.unwrap_or(u64::MAX))
                .or_insert(Default::default());
            candle.gc_id = gc_id.unwrap_or(u64::MAX);
            match &evt {
                JfrEvent::G1GarbageCollection { values } => {
                    candle.collection_type = values.type_;
                }
                JfrEvent::PromoteObjectOutsidePLAB { values } => {
                    if values.tenured {
                        candle.tenured += values.object_size;
                    }
                }
                JfrEvent::PromoteObjectInNewPLAB { values } => {
                    if values.tenured {
                        candle.tenured += values.plab_size;
                    }
                }
                JfrEvent::G1HeapSummary { values } => match values.when {
                    jfr::GCWhen::Before => {
                        candle.young_before = values.eden_used;
                        candle.survivors_before = values.survivor_used;
                    }
                    jfr::GCWhen::After => {
                        candle.young_after = values.eden_used;
                        candle.survivors_after = values.survivor_used;
                    }
                },
                JfrEvent::GCHeapSummary { values } => match values.when {
                    jfr::GCWhen::Before => candle.before_gc = values.heap_used,
                    jfr::GCWhen::After => {
                        candle.gc_id = values.gc_id;
                        candle.after_gc = values.heap_used;
                    }
                },
                JfrEvent::GarbageCollection { values } => candle.gc_name = values.name.clone(),
                JfrEvent::GCPhasePause { values } => candle.gc_pause_name = values.name.clone(),
                JfrEvent::YoungGarbageCollection { values } => {
                    candle.tenuring_threshold = values.tenuring_threshold
                }
                _ => {}
            }
        }
        let mut x_axis: Vec<f64> = Vec::new();
        let mut heap: Vec<u64> = Vec::new();
        let mut young: Vec<u64> = Vec::new();
        let mut tenured: Vec<u64> = Vec::new();
        let mut survivors: Vec<u64> = Vec::new();
        let mut text_array = Vec::<String>::new();
        let mut ix = 0;
        for (gc_id, candle) in gc_id_to_candle {
            if gc_id == u64::MAX {
                continue;
            }
            if collection_type_filter.contains(&candle.collection_type) {
                continue;
            }
            const FACTOR: f64 = 2.3f64;
            let tenured_bytes = candle.tenured;
            let gc_id_x_axis = ix as f64 * FACTOR;
            x_axis.push(gc_id_x_axis);
            heap.push(candle.before_gc - candle.young_before);
            if tenured_bytes > candle.young_before + candle.survivors_before {
                println!(
                    "weird, we see {} tenure candidates but >{} tenured in gc {}",
                    candle.young_before + candle.survivors_before,
                    tenured_bytes,
                    candle.gc_id
                );
                young.push(candle.young_before);
                tenured.push(0);
                survivors.push(candle.survivors_before);
            } else {
                let young_before = candle.young_before.saturating_sub(tenured_bytes);
                let survivors_before =
                    candle.survivors_before - tenured_bytes.saturating_sub(candle.young_before);

                young.push(young_before);
                tenured.push(tenured_bytes);
                survivors.push(survivors_before);
            }
            text_array.push(format!("[{}] before gc", gc_id));
            graphs.gcs_labels.push(candle.title());
            graphs.gcs_ticks.push(gc_id_x_axis as f64);

            let gc_id_x_axis = ix as f64 * FACTOR + 1f64;
            x_axis.push(gc_id_x_axis);
            if candle.after_gc < tenured_bytes {
                heap.push(candle.after_gc - candle.young_after);
                young.push(candle.young_after);
                tenured.push(0);
                survivors.push(candle.survivors_after);
            } else {
                heap.push(candle.after_gc - candle.young_after - tenured_bytes);
                young.push(candle.young_after);
                tenured.push(tenured_bytes);
                survivors.push(candle.survivors_after);
            }
            text_array.push(format!("[{}] after gc", gc_id));
            graphs.gcs_labels.push(candle.title());
            ix += 1;
        }
        graphs.gcs.push(
            *Bar::new(x_axis.clone(), heap)
                .name("heap")
                .text_array(text_array.clone()),
        );
        graphs.gcs.push(
            *Bar::new(x_axis.clone(), tenured)
                .name("tenured")
                .text_array(text_array.clone()),
        );
        graphs.gcs.push(
            *Bar::new(x_axis.clone(), young)
                .name("young")
                .text_array(text_array.clone()),
        );
        graphs.gcs.push(
            *Bar::new(x_axis.clone(), survivors)
                .name("survivors")
                .text_array(text_array),
        );
        let tenure_vec = &self
            .recording
            .events
            .iter()
            .filter_map(|e| {
                if let JfrEvent::TenuringDistribution { values } = e {
                    Some(values)
                } else {
                    None
                }
            })
            .collect_vec();

        let by_gc_id = tenure_vec.iter().chunk_by(|e| e.gc_id);
        for (_gc_id, tenures) in by_gc_id.into_iter() {
            let mut ages = Vec::<u64>::new();
            let mut sizes = Vec::<u64>::new();
            for tenure in tenures {
                ages.push(tenure.age);
                sizes.push(tenure.size);
            }
            let trace = Scatter::new(ages, sizes);
            graphs.ages.push(*trace);
        }
        graphs
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut cmd = Command::new("jfr");
    cmd.args(["print", "--json", &args.jfr_file]);
    let jfr_out = cmd.output().expect("cannnot run jfr");
    if !jfr_out.status.success() {
        std::io::stderr().write_all(&jfr_out.stderr).unwrap();
        return;
    }
    let jfr_evt: JfrMain = serde_json::from_slice(&jfr_out.stdout).expect("cannot parse jfr JSON");
    let app = Router::new()
        .route("/ages", get(ages))
        .route("/", get(index))
        .route("/plotly-2.32.0.min.js", get(plotlyjs))
        .route("/tex-svg.js", get(tex))
        .route("/favicon.ico", get(favicon))
        .with_state(jfr_evt);
    let listener = tokio::net::TcpListener::bind(format!("{}", args.addr))
        .await
        .unwrap();
    println!("listening on {}", args.addr);
    // we don't care if it fails
    if args.webbrowser {
        let _ = webbrowser::open(format!("http://{}", args.addr).as_str());
    }
    axum::serve(listener, app).await.unwrap();
    println!("Hello, world!");
}

async fn ages(
    State(jfr_main): State<JfrMain>,
    Query(params): Query<Vec<(String, String)>>,
) -> Json<Vec<Plot>> {
    let mut ages = Plot::new();
    let filter: HashSet<CollectionType> = params
        .into_iter()
        .filter_map(|(key, val)| -> Option<CollectionType> {
            if key == "collection_type_filter" {
                let x: Option<CollectionType> =
                    serde_json::from_str(format!(r#""{}""#, val).as_str()).ok();
                println!("filter {}<>{} = {:?}", format!(r#""{}""#, val), val, x);
                serde_json::from_str(format!(r#""{}""#, val).as_str()).ok()
            } else {
                None
            }
        })
        .collect();
    /*let filter: HashSet<CollectionType> = params
    .get_vec("collection_type_filter")
    .map(|e| serde_json::from_str::<CollectionType>(e).unwrap())
    .collect();*/
    let graphs = jfr_main.to_graphs(filter);
    for trace in graphs.ages {
        ages.add_trace(Box::new(trace));
    }
    let mut gc = Plot::new();
    for trace in graphs.gcs {
        gc.add_trace(Box::new(trace));
    }
    gc.set_layout(
        Layout::new()
            .x_axis(
                Axis::new()
                    .tick_values(graphs.gcs_ticks)
                    .tick_text(graphs.gcs_labels)
                    .tick_mode(common::TickMode::Array),
            )
            .bar_mode(plotly::layout::BarMode::Stack),
    );
    Json(Vec::from([ages, gc]))
}

async fn index(State(_state): State<JfrMain>) -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}
async fn plotlyjs(State(_state): State<JfrMain>) -> Html<&'static str> {
    Html(include_str!("../assets/plotly-2.32.0.min.js"))
}
async fn tex(State(_state): State<JfrMain>) -> Html<&'static str> {
    Html(include_str!("../assets/tex-svg.js"))
}
async fn favicon(State(_state): State<JfrMain>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/png")],
        include_bytes!("../assets/favicon.ico"),
    )
}
