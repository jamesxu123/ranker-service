mod elo;
mod scheduler;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use scheduler::{Item, Judge, MatchPair, MatchWinner};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = scheduler::SchedulerState::new();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/judge", post(create_judge).get(get_judges))
        .route("/item", post(create_item).get(get_items))
        .route("/scheduler_start", post(start_matchmaking))
        .route("/matches", get(get_matches))
        .route("/matches/for_judge", post(request_match_for_judge))
        .route("/matches/judge", post(judge_match))
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    tracing::debug!("listening on {}", addr);
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

async fn get_judges(
    State(state): State<scheduler::SchedulerState>,
) -> (StatusCode, Json<Vec<Judge>>) {
    (StatusCode::OK, Json(state.get_judges()))
}

async fn create_judge(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    State(state): State<scheduler::SchedulerState>,
    Json(payload): Json<CreateJudge>,
) -> (StatusCode, Json<Judge>) {
    // insert your application logic here
    let user = Judge::new(payload.email);
    state.add_judge(user.clone());
    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateJudge {
    email: String,
}

async fn create_item(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    State(state): State<scheduler::SchedulerState>,
    Json(payload): Json<CreateItem>,
) -> (StatusCode, &'static str) {
    // insert your application logic here
    let item = Item::new(payload.name, payload.location, payload.description);
    state.add_item(item);
    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, "success")
}

async fn get_items(
    State(state): State<scheduler::SchedulerState>,
) -> (StatusCode, Json<Vec<Box<Item>>>) {
    (StatusCode::OK, Json(state.get_items()))
}

#[derive(Deserialize)]
struct CreateItem {
    name: String,
    location: String,
    description: String,
}

#[derive(Deserialize)]
struct SeedStart {
    n: usize,
}

async fn start_matchmaking(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    State(state): State<scheduler::SchedulerState>,
    Json(payload): Json<SeedStart>,
) -> (StatusCode, &'static str) {
    // insert your application logic here
    let status = state.seed_start(payload.n);
    if !status {
        return (
            StatusCode::BAD_REQUEST,
            "failed to start (likely already started)",
        );
    }
    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, "success")
}

async fn get_matches(
    State(state): State<scheduler::SchedulerState>,
) -> (StatusCode, Json<Vec<Arc<MatchPair>>>) {
    if let Ok(matches) = state.get_match_pairs() {
        (StatusCode::OK, Json(matches))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(vec![]))
    }
}

#[derive(Serialize)]
enum ValOrError<T> {
    Value(T),
    Error(String),
}

async fn request_match_for_judge(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    State(state): State<scheduler::SchedulerState>,
    Json(payload): Json<Judge>,
) -> (StatusCode, Json<ValOrError<Arc<MatchPair>>>) {
    // insert your application logic here
    let matchpair = state.give_judge_next_match(&payload);
    match matchpair {
        Ok(mp) => (StatusCode::CREATED, Json(ValOrError::Value(mp))),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ValOrError::Error(err.to_string())),
        ),
    }
}

#[derive(Deserialize)]
struct JudgeMatch {
    judge: Judge,
    match_id: String,
    winner: MatchWinner,
}

async fn judge_match(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    State(state): State<scheduler::SchedulerState>,
    Json(payload): Json<JudgeMatch>,
) -> (StatusCode, &'static str) {
    // insert your application logic here
    match state.judge_match(&payload.judge, &payload.match_id, payload.winner) {
        true => (StatusCode::OK, "judged"),
        false => (StatusCode::INTERNAL_SERVER_ERROR, "yeah does not exist"),
    }
}
