mod shared_data;

use crate::shared_data::SharedData;

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::server::conn::AddrStream;
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};

use serde_json::Value;

#[macro_use] extern crate serde_derive;

#[tokio::main]
async fn main() {
    let data = Arc::new(Mutex::new(SharedData::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let service = make_service_fn(move |_client: &AddrStream| {
        let data = data.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let data = data.clone();
                handle_request(req, data)
            }))
        }
    });

    let server = Server::bind(&addr).serve(service);

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }
}

async fn handle_request(req: Request<Body>, data: Arc<Mutex<SharedData>>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/items") => {
            let params: HashMap<String, String> = req
                .uri()
                .query()
                .map(|val| {
                    url::form_urlencoded::parse(val.as_bytes())
                    .into_owned()
                    .collect()
                }).unwrap_or_else(HashMap::new);

            let response = get_items(data.clone(), params);
            Ok(response)
        },
        (&Method::POST, "/items") => {
            let value = parse_body_to_json(req.into_body()).await;
            let response = match value {
                Some(value) => post_items(data.clone(), value),
                None => {
                    let mut response = Response::default();
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                    response
                },
            };
            
            Ok(response)
        },
        (&Method::POST, "/items/delete") => {
            let value = parse_body_to_json(req.into_body()).await;
            let response = match value {
                Some(value) => delete_item(data.clone(), value),
                None => {
                    let mut response = Response::default();
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                    response
                },
            };
            
            Ok(response)
        },
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

fn get_items(shared_data: Arc<Mutex<SharedData>>, params: HashMap<String, String>) -> Response<Body> {
    let table = params.get("table");

    if table.is_none() {
        return create_400_response("missing required field: 'table'".to_string());
    }

    let table_int = match table.unwrap().parse::<u64>() {
        Ok(table_int) => table_int,
        Err(_) => {
            return create_400_response("'table' must be an unassigned integer!".to_string());
        },
    };
    
    let data = shared_data.lock().unwrap();
    let items = data.get_tables().get(&table_int);

    let order_id = params.get("order_id");

    let mut response = Response::default();
    let json_string: String;
    *response.status_mut() = StatusCode::OK;

    if order_id.is_none() {
        // Logic for setting response body for when no order_id was provided

        json_string = match items {
            Some(items) => serde_json::to_string(items).unwrap(),
            None => "[]".to_string(),
        };
    } else {
        // Logic for setting response body for when an order_id was provided

        let order_id_int = match order_id.unwrap().parse::<u64>() {
            Ok(order_id_int) => order_id_int,
            Err(_) => {
                return create_400_response("'order_id' must be an unassigned integer!".to_string());
            },
        };

        if items.is_none() {
            return create_400_response(format!("order {} doesn't exist at table {}", order_id_int, table_int).to_string());
        } else {
            let order = items.unwrap().into_iter().find(|&o| o.get_order_id() == order_id_int);
            if order.is_none() {
                return create_400_response(format!("order {} doesn't exist at table {}", order_id_int, table_int).to_string());
            }

            json_string = serde_json::to_string(&order).unwrap();
        }
    }

    *response.body_mut() = hyper::Body::from(json_string);
    response
}

fn post_items(shared_data: Arc<Mutex<SharedData>>, value: Value) -> Response<Body> {
    let mut response = Response::default();

    if value["table"].is_null() || value["items"].is_null() {
        return create_400_response("missing required fields: 'table' and\\or 'items'".to_string());
    }

    let table = value["table"].as_u64();
    if table.is_none() {
        return create_400_response("'table' must be an unassigned integer!".to_string());
    }

    let items = value["items"].as_array();
    if items.is_none() {
        return create_400_response("'items' must be an array of strings!".to_string());
    }

    // Converts items from Vec<Value> to Vec<String> as needed by SharedData
    let items = items
        .unwrap()
        .into_iter()
        .map(|x| {
            let temp = x.as_str();
            if temp.is_none() { return Err(()); }
            Ok(temp.unwrap().to_string())
        })
        .collect::<Result<Vec<_>, ()>>();

    let items = match items {
        Ok(items) => items,
        Err(_) => {
            return create_400_response("'items' must be an array of strings!".to_string());
        },
    };

    shared_data.lock().unwrap().add_items(table.unwrap(), items);

    *response.status_mut() = StatusCode::OK;
    response
}

fn delete_item(shared_data: Arc<Mutex<SharedData>>, value: Value) -> Response<Body>  {
    if value["table"].is_null() || value["order_id"].is_null() {
        return create_400_response("missing required fields: 'table' and\\or 'order_id'".to_string());
    }

    let table = value["table"].as_u64();
    if table.is_none() {
        return create_400_response("'table' must be an unassigned integer!".to_string());
    }

    let order_id = value["order_id"].as_u64();
    if order_id.is_none() {
        return create_400_response("'order_id' must be an unassigned integer!".to_string());
    }

    let message = shared_data.lock().unwrap().remove_item(table.unwrap(), order_id.unwrap());
    match message {
        Some(message) => {
            return create_400_response(message)
        },
        None => {
            let mut response = Response::default();
            *response.status_mut() = StatusCode::OK;
            response
        },
    }
}

async fn parse_body_to_json(body: Body) -> Option<Value> {
    let bytes = hyper::body::to_bytes(body).await;
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(_) => return None,
    };

    let value = serde_json::from_slice(&bytes);
    let value: Value = match value {
        Ok(value) => value,
        Err(_) => return None,
    };

    Some(value) 
}

fn create_400_response(message: String) -> Response<Body> {
    let mut response = Response::default();
    *response.body_mut() = hyper::Body::from(message);
    *response.status_mut() = StatusCode::BAD_REQUEST;
    return response
}
