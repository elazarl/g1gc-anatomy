pub mod jfr;
use crate::jfr::{CollectionType, JfrEvent, JfrMain};
use std::{collections::BTreeMap, io::Write, process::Command};

use axum::{
    extract::State,
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
}

#[derive(Default, Clone, Debug)]
struct Graphs {
    ages: Vec<Scatter<u64, u64>>,
    gcs: Vec<Bar<f64, u64>>,
    gcs_labels: Vec<String>,
    gcs_ticks: Vec<f64>,
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
    let mut graphs: Graphs = Default::default();
    let jfr_evt: JfrMain = serde_json::from_slice(&jfr_out.stdout).expect("cannot parse jfr JSON");
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
        collection_type: CollectionType,
    }
    let mut gc_id_to_candle: BTreeMap<u64, Candle> = BTreeMap::new();
    for evt in &jfr_evt.recording.events {
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
            _ => {}
        }
    }
    let mut x_axis: Vec<f64> = Vec::new();
    let mut heap: Vec<u64> = Vec::new();
    let mut young: Vec<u64> = Vec::new();
    let mut tenured: Vec<u64> = Vec::new();
    let mut survivors: Vec<u64> = Vec::new();
    let mut text_array = Vec::<String>::new();
    for (gc_id, candle) in gc_id_to_candle {
        if gc_id == u64::MAX {
            continue;
        }
        const FACTOR: f64 = 2.3f64;
        let tenured_bytes = candle.tenured;
        let gc_id_x_axis = candle.gc_id as f64 * FACTOR;
        x_axis.push(gc_id_x_axis);
        heap.push(candle.before_gc - candle.young_before);
        assert!(tenured_bytes <= candle.young_before + candle.survivors_before);
        let young_before = candle.young_before.saturating_sub(tenured_bytes);
        let survivors_before =
            candle.survivors_before - tenured_bytes.saturating_sub(candle.young_before);

        young.push(young_before);
        tenured.push(tenured_bytes);
        survivors.push(survivors_before);
        text_array.push(format!("[{}] before gc", gc_id));
        graphs
            .gcs_labels
            .push(format!("{:?}", candle.collection_type));
        graphs.gcs_ticks.push(gc_id_x_axis as f64);

        let gc_id_x_axis = candle.gc_id as f64 * FACTOR + 1f64;
        x_axis.push(gc_id_x_axis);
        assert!(candle.after_gc >= tenured_bytes);
        heap.push(candle.after_gc - candle.young_after - tenured_bytes);
        young.push(candle.young_after);
        tenured.push(tenured_bytes);
        survivors.push(candle.survivors_after);
        text_array.push(format!("[{}] after gc", gc_id));
        graphs
            .gcs_labels
            .push(format!("{:?}", candle.collection_type));
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
    let tenure_vec = jfr_evt
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
    let app = Router::new()
        .route("/ages", get(ages))
        .route("/", get(index))
        .route("/plotly-2.32.0.min.js", get(plotlyjs))
        .route("/tex-svg.js", get(tex))
        .route("/favicon.ico", get(favicon))
        .with_state(graphs);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    println!("Hello, world!");
}

async fn ages(State(graphs): State<Graphs>) -> Json<Vec<Plot>> {
    let mut ages = Plot::new();
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

async fn index(State(_state): State<Graphs>) -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}
async fn plotlyjs(State(_state): State<Graphs>) -> Html<&'static str> {
    Html(include_str!("../assets/plotly-2.32.0.min.js"))
}
async fn tex(State(_state): State<Graphs>) -> Html<&'static str> {
    Html(include_str!("../assets/tex-svg.js"))
}
async fn favicon(State(_state): State<Graphs>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/png")],
        include_bytes!("../assets/favicon.ico"),
    )
}
