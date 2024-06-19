use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::SERVER_CONFIG;
use crate::database::Database;
use crate::http::request::Request;
use crate::http::response::Response;
use crate::results::telemetry::{draw_result, record_result};

pub async fn telemetry_record_route(database : &mut Arc<Mutex<dyn Database + Send>>,request : &Request) -> Response {
    let server_config = SERVER_CONFIG.get().unwrap();
    match server_config.database_type.as_str() {
        "none" => {
            Response::res_200("Telemetry Disabled.")
        }
        _ => {
            let record_result = record_result(request,database).await;
            match record_result {
                Ok(uuid) => {
                    let response_content = format!("id {}",uuid);
                    Response::res_200(&response_content)
                }
                Err(_) => {
                    Response::res_500()
                }
            }
        }
    }
}

pub async fn show_result_route (database : &mut Arc<Mutex<dyn Database + Send>>, params: &HashMap<String, String>) -> Response {
    let result_id = params.get("id");
    match result_id {
        Some(result_id) => {
            let mut db = database.lock().await;
            let fetched_result = db.fetch_by_uuid(result_id);
            match fetched_result {
                Ok(fetched_result) => {
                    match fetched_result {
                        Some(fetched_telemetry_data) => {
                            let image = draw_result(&fetched_telemetry_data);
                            drop(fetched_telemetry_data);
                            Response::res_200_img(&image)
                        }
                        None => {
                            Response::res_404()
                        }
                    }
                }
                Err(_) => {
                    Response::res_404()
                }
            }
        }
        None => {
            Response::res_400()
        }
    }
}