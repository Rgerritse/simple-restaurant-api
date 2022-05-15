use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::server::conn::AddrStream;
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};

use serde_json::Value;
use rand::Rng;

#[macro_use] extern crate serde_derive;

#[derive(Serialize, Deserialize, Debug)]
struct Order {
    order_id: u64,
    item: String,
    mins_to_cook: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct SharedData {
    order_count: u64,
    tables: HashMap<u64, Vec<Order>>,
}

impl SharedData {
    fn add_items(&mut self, table: u64, items: Vec<String>) {
        let mut rng = rand::thread_rng();
        let mut orders = items.iter().map(|item| {
            let order = Order {
                order_id: self.order_count,
                item: item.to_string(),
                mins_to_cook:  rng.gen_range(5..16)
            };
            self.order_count += 1;
            return order
        }).collect::<Vec<Order>>();

        match self.tables.get_mut(&table) {
            Some(v) => {
                v.append(&mut orders)
            }
            None => {     
                self.tables.insert(table, orders);  
            }
        };
    }

    fn remove_item(&mut self, table: u64, order_id: u64) {
        let orders = self.tables.get_mut(&table).unwrap();
        println!("orders bef: {:?}", orders);
        let index = orders.iter().position(|o| o.order_id == order_id).unwrap();

        println!("index: {}", index);
        orders.remove(index);
        println!("order af {:?}", orders);
    }
}

async fn handle_request(req: Request<Body>, data: Arc<std::sync::Mutex<SharedData>>) -> Result<Response<Body>, hyper::Error> {
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
            let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
            let value: Value = serde_json::from_slice(&body_bytes).unwrap();
            let response = post_items(data.clone(), value);
            Ok(response)
        },
        (&Method::POST, "/items/delete") => {
            let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
            let value: Value = serde_json::from_slice(&body_bytes).unwrap();
            let response = delete_item(data.clone(), value);
            Ok(response)
        },
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
async fn main() {
    let data = Arc::new(Mutex::new(SharedData { order_count: 0, tables: HashMap::new() })) ;

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

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

fn get_items(shared_data: Arc<Mutex<SharedData>>, params: HashMap<String, String>) -> Response<Body> {
    let mut response = Response::default();
    let table = params.get("table");

    if table.is_none() {
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response;
    }

    let table_int = table.unwrap().parse::<u64>().unwrap();
    
    let data = shared_data.lock().unwrap();
    let items = data.tables.get(&table_int).unwrap();
    let json_string = serde_json::to_string(items).unwrap();

    *response.body_mut() = hyper::Body::from(json_string);
    *response.status_mut() = StatusCode::OK;
    response
}

fn post_items(shared_data: Arc<Mutex<SharedData>>, value: Value) -> Response<Body> {
    let mut response = Response::default();

    let table = value["table"].as_u64();
    let items = value["items"].as_array();
    if table.is_none() || items.is_none() {
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

    let table = value["table"].as_u64();
    let order_id = value["order_id"].as_u64();

    if table.is_none() || order_id.is_none() {
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response
    }

    shared_data.lock().unwrap().remove_item(table.unwrap(), order_id.unwrap());

    *response.status_mut() = StatusCode::OK;
    response
}
