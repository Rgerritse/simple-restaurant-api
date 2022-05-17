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
    let data = Arc::new(Mutex::new(SharedData { order_counter: 0, tables: HashMap::new() })) ;

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
    let mut response = Response::default();
    let table = params.get("table");

    if table.is_none() {
        *response.body_mut() = hyper::Body::from("missing required field: 'table'");
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response;
    }

    let table_int = table.unwrap().parse::<u64>();

    let table_int = match table_int {
        Ok(table_int) => table_int,
        Err(_) => {
            *response.body_mut() = hyper::Body::from("'table' must be an unassigned integer!");
            *response.status_mut() = StatusCode::BAD_REQUEST;
            return response;
        },
    };
    
    let data = shared_data.lock().unwrap();
    let items = data.tables.get(&table_int);

    let json_string = match items {
        Some(items) => serde_json::to_string(items).unwrap(),
        None => String::from("")
    };

    *response.body_mut() = hyper::Body::from(json_string);
    *response.status_mut() = StatusCode::OK;
    response
}

fn post_items(shared_data: Arc<Mutex<SharedData>>, value: Value) -> Response<Body> {
    let mut response = Response::default();

    if value["table"].is_null() || value["items"].is_null() {
        *response.body_mut() = hyper::Body::from("missing required fields: 'table' and\\or 'items'");
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response
    }

    let table = value["table"].as_u64();
    if table.is_none() {
        *response.body_mut() = hyper::Body::from("'table' must be an unassigned integer!");
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response
    }

    let items = value["items"].as_array();
    if items.is_none() {
        *response.body_mut() = hyper::Body::from("'items' must be an array!");
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response
    }

    let items = items
        .unwrap()
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>();

        shared_data.lock().unwrap().add_items(table.unwrap(), items);

    *response.status_mut() = StatusCode::OK;
    response
}

fn delete_item(shared_data: Arc<Mutex<SharedData>>, value: Value) -> Response<Body>  {
    let mut response = Response::default();

    if value["table"].is_null() || value["order_id"].is_null() {
        *response.body_mut() = hyper::Body::from("missing required fields: 'table' and\\or 'order_id'");
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response
    }

    let table = value["table"].as_u64();
    if table.is_none() {
        *response.body_mut() = hyper::Body::from("'table' must be an unassigned integer!");
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response
    }

    let order_id = value["order_id"].as_u64();
    if order_id.is_none() {
        *response.body_mut() = hyper::Body::from("'order_id' must be an unassigned integer!");
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response
    }

    let message = shared_data.lock().unwrap().remove_item(table.unwrap(), order_id.unwrap());
    match message {
        Some(message) => {
            *response.body_mut() = hyper::Body::from(message);
            *response.status_mut() = StatusCode::BAD_REQUEST;
        },
        None => {
            *response.status_mut() = StatusCode::OK;
        },
    }

    response
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
