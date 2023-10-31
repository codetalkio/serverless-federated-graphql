use apollo_router::services::supergraph;
use apollo_router::Configuration;
use apollo_router::TestHarness;
use lambda_http::{run, service_fn, Error, IntoResponse, Request};
use reqwest::header::CONTENT_TYPE;
use std::env;
use std::fs;
use std::sync::Arc;
use tower::util::ServiceExt;
use tracing::info;

async fn handle_request(event: Request) -> Result<impl IntoResponse, Error> {
    let config_path = env::var("APOLLO_ROUTER_CONFIG_PATH").unwrap_or("./router.yaml".to_string());
    let schema_path =
        env::var("APOLLO_ROUTER_SUPERGRAPH_PATH").unwrap_or("./supergraph.graphql".to_string());
    let config = fs::read_to_string(config_path)?;
    let schema = fs::read_to_string(schema_path)?;
    let configuration = serde_yaml::from_str::<Configuration>(&config).unwrap();

    let body = event.body();
    let event_payload = std::str::from_utf8(body).expect("invalid utf-8 sequence");
    info!("👉 Proxying request to router: {:?}", event_payload);

    let request = supergraph::Request::fake_builder()
        .header(CONTENT_TYPE, "application/json")
        // TODO: Extract these from the event_payload.
        .query("query ExampleQuery { products { __typename id } }")
        .operation_name("ExampleQuery")
        // Construct variables as JsonMap<ByteString, Value>.
        // .variables("")
        .build()
        .unwrap();

    let supergraph = TestHarness::builder()
        .configuration(Arc::new(configuration))
        .schema(&schema)
        // Without this all subgraphs get an empty response by default.
        .with_subgraph_network_requests()
        .build_router()
        .await?;
    let mut response = supergraph.oneshot(request.try_into().unwrap()).await?;

    let resp: serde_json::Value = serde_json::from_slice(
        response.next_response().await.unwrap().unwrap().to_vec().as_slice(),
    )?;
    Ok(resp)

    // let resp: apollo_router::graphql::Response = serde_json::from_slice(
    //     response.next_response().await.unwrap().unwrap().to_vec().as_slice(),
    // )?;
    // info!("👉 Deserialized Response: {:?}", resp);
    // // let status = resp.status();
    // // let payload = resp.json::<serde_json::Value>().await?;
    // let payload = serde_json::to_string(&resp).unwrap();

    // TODO: Return whitelisted headers from the Router response.
    // Ok(payload)
}

async fn handler() -> Result<(), Error> {
    // Set up the Lambda event handler.
    run(service_fn(|event: Request| async { handle_request(event).await })).await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // disable coloring.
        .with_ansi(false)
        .init();
    handler().await
}
