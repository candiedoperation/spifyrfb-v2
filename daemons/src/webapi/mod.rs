/*
    SpifyRFB - Modern RFB Server implementation using Rust
    Copyright (C) 2023  Atheesh Thirumalairajan

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#[cfg(target_os = "windows")]
use crate::windows;

use serde::{Serialize, Deserialize};
use serde_json::json;
use axum::{Router, routing::get, response::{Response, IntoResponse}, http::StatusCode};

#[derive(Serialize, Deserialize)]
pub struct WebApiSession {
    pub(crate) pid: u32,
    pub(crate) ip: String,
    pub(crate) ws: String,
    pub(crate) ws_secure: bool,
    pub(crate) username: String,
    pub(crate) logontime: i64
}

fn get_routes() -> Router {
    Router::new()
    .route("/", get(root))
    .route("/api/status", get(get_status))
    .route("/api/sessions", get(get_sessions))
}

async fn root() -> Response {
    (
        StatusCode::OK,
        axum::response::Html("<h1>SpifyRFB Daemon WebAPI</h1><p>SpifyRFB WebClients can Understand this page</p>")
    ).into_response()
}

async fn get_status() -> Response {
    /* Define Hostname */
    #[allow(unused_mut)]
    let mut hostname = "Hostname Unknown".to_string();

    #[cfg(target_os = "windows")]
    { hostname = windows::get_hostname(); }

    /* Send Reponse */
    (StatusCode::OK, axum::response::Json(json!({
        "online": true,
        "hostname": hostname
    }))).into_response()
}

async fn get_sessions() -> Response {
    /* Define Active Sessions */
    #[allow(unused_mut)]
    let mut sessions = json!([]);
   
    #[cfg(target_os = "windows")]
    { sessions = json!(windows::webapi_getsessions()) }
    
    /* Send Response */
    (StatusCode::OK, axum::response::Json(sessions)).into_response()
}

pub async fn create() {
    axum::Server::bind(&"0.0.0.0:10000".parse().unwrap())
    .serve(get_routes().into_make_service())
    .await
    .unwrap();
}