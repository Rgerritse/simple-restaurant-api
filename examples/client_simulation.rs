use hyper::{Client, Body, Request, Method};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    let base_url = "http://127.0.0.1:3000";

    let client = Client::new();

    // Adding items to table 1
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("{}/items", base_url))
        .header("content-type", "application/json")
        .body(Body::from(r#"{ "table": 1, "items": ["cola", "tea", "pizza"] }"#))?;

    client.request(req).await?;

    let uri = format!("{}/items?table=1", base_url).parse()?;
    let res = client.get(uri).await?;
    println!("\nTable 1 after post request:");
    print_response_body(res.into_body()).await;

    // Removing item from table 1
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("{}/items/delete", base_url))
        .header("content-type", "application/json")
        .body(Body::from(r#"{ "table": 1, "order_id": 1 }"#))?;

    client.request(req).await?;

    let uri = format!("{}/items?table=1", base_url).parse()?;
    let res = client.get(uri).await?;
    println!("\nTable 1 after delete request:");
    print_response_body(res.into_body()).await;

    // Request single item from table 1
    let uri = format!("{}/items?table=1&order_id=2", base_url).parse()?;

    let res = client.get(uri).await?;
    println!("\nGet request for item where table=1 and order_id=2:");
    print_response_body(res.into_body()).await;
    
    // ---------------------
    // 5 Parallel requests
    // ---------------------

    // Add items table 1 request
    let request1 = async {
        let req = Request::builder()
        .method(Method::POST)
        .uri(format!("{}/items", base_url))
        .header("content-type", "application/json")
        .body(Body::from(r#"{ "table": 1, "items": ["sandwich"] }"#));

        let res = client.request(req.unwrap()).await?;
        hyper::body::to_bytes(res.into_body()).await
    };

    // Add items table 2 request 
    let request2 = async {
        let req = Request::builder()
        .method(Method::POST)
        .uri(format!("{}/items", base_url))
        .header("content-type", "application/json")
        .body(Body::from(r#"{ "table": 2, "items": ["tea"] }"#));

        let res =client.request(req.unwrap()).await?;
        hyper::body::to_bytes(res.into_body()).await
    };

    // Add items table 2 request 
    let request3 = async {
        let req = Request::builder()
        .method(Method::POST)
        .uri(format!("{}/items", base_url))
        .header("content-type", "application/json")
        .body(Body::from(r#"{ "table": 2, "items": ["pizza"] }"#));

        let res =client.request(req.unwrap()).await?;
        hyper::body::to_bytes(res.into_body()).await
    };

    // Remove item table 1 request
    let request4 = async {
        let req = Request::builder()
        .method(Method::POST)
        .uri(format!("{}/items/delete", base_url))
        .header("content-type", "application/json")
        .body(Body::from(r#"{ "table": 1, "order_id": 2 }"#));

        let res = client.request(req.unwrap()).await?;
        hyper::body::to_bytes(res.into_body()).await
    };

    // Get items table 1 request
    let request5 = async {
        let uri = format!("{}/items?table=1", base_url).parse();
        let res = client.get(uri.unwrap()).await?;
        hyper::body::to_bytes(res.into_body()).await
    };

    // Wait for the 5 requests
    futures::try_join!(request1, request2, request3, request4, request5)?;

    let uri = format!("{}/items?table=1", base_url).parse()?;
    let res = client.get(uri).await?;
    println!("\nTable 1 after parallel request:");
    print_response_body(res.into_body()).await;

    let uri = format!("{}/items?table=2", base_url).parse()?;
    let res = client.get(uri).await?;
    println!("\nTable 2 after parallel requests:");
    print_response_body(res.into_body()).await;

    Ok(())
}

async fn print_response_body(body: Body) {
    let body_bytes = hyper::body::to_bytes(body).await;
    let body_str = String::from_utf8(body_bytes.unwrap().to_vec()).unwrap();
    println!("{}", body_str);
}