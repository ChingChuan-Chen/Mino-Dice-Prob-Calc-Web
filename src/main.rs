use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

mod simulation;

use simulation::{SimulationParams, run_simulation};

#[derive(Deserialize)]
struct SimulateRequest {
    number_players: usize,
    round_number: usize,
    order: usize,
    own_dices: Vec<String>,
    number_experiments: usize,
}

#[derive(Serialize)]
struct SimulateResponse {
    win_count_probs: Vec<f64>,
    labels: Vec<String>,
}

async fn simulate_handler(
    Json(req): Json<SimulateRequest>,
) -> Json<SimulateResponse> {
    let params = SimulationParams {
        number_players: req.number_players,
        round_number: req.round_number,
        order: req.order,
        own_dices: req.own_dices,
        number_experiments: req.number_experiments,
    };

    let win_counts = run_simulation(&params);

    let total: usize = win_counts.values().sum();
    let mut labels = Vec::new();
    let mut probs = Vec::new();
    for i in 0..=params.round_number {
        labels.push(i.to_string());
        let count = win_counts.get(&i).copied().unwrap_or(0);
        probs.push(count as f64 / total as f64 * 100.0);
    }

    Json(SimulateResponse {
        win_count_probs: probs,
        labels,
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/simulate", post(simulate_handler))
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
