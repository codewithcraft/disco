use axum::{
    extract::{Path, State},
    response::Html,
    routing::{get, post},
    Form, Router,
};
use rusty_jsc::JSContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize)]
struct Save {
    script: String,
    key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AppState {
    scripts: Mutex<HashMap<String, String>>,
}

#[tokio::main]
async fn main() {
    let shared_state: Arc<AppState> = match std::path::Path::new("config.json").is_file() {
        true => {
            let contents =
                std::fs::read_to_string("config.json").expect("Could not read config");
            Arc::new(serde_json::from_str(&contents).expect("Could not parse config"))
        }
        false => Arc::new(AppState {
            scripts: Mutex::new(HashMap::new()),
        }),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/:key", get(handler_run))
        .route("/", post(handler_set))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let source = include_str!("../web/index.html");
    let list = serde_json::to_string(&state.scripts).unwrap();
    let result = source.replace("//--list--", &format!("const list = {}", &list));
    Html(result)
}

async fn handler_run(Path(key): Path<String>, State(state): State<Arc<AppState>>) -> String {
    let mut context = JSContext::default();

    let scripts = state.scripts.lock().expect("Could not unlock mutex");
    let script = &scripts.get(&key);

    let script = match script {
        Some(script) => script,
        None => return String::from("Not Found"),
    };

    let result =
        match context.evaluate_script(&format!("function run() {{ {} }}; run();", script), 1) {
            Ok(value) => format!("{}", value.to_string(&context).unwrap()),
            Err(error) => format!("Error: {}", error.to_string(&context).unwrap()),
        };

    drop(scripts);

    result
}

async fn handler_set(State(state): State<Arc<AppState>>, Form(save): Form<Save>) -> Html<String> {
    let source = include_str!("../web/index.html");
    let mut scripts = state.scripts.lock().expect("Could not unlock mutex");
    scripts.insert(save.key, save.script);
    drop(scripts);
    let config = serde_json::to_string(&state).unwrap();
    std::fs::write("config.json", &config).expect("Could not write config.json");
    let list = serde_json::to_string(&state.scripts).unwrap();
    let result = source.replace("//--list--", &format!("const list = {}", &list));
    Html(result)
}
