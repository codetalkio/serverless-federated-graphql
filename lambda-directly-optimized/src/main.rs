use apollo_router::services::{router, supergraph};
use apollo_router::{Configuration, TestHarness};
use lambda_http::{run, service_fn, Error, IntoResponse, Request};
use reqwest::header::CONTENT_TYPE;
use std::env;
use std::fs;
use std::sync::{Arc, Mutex};
use tower::Service;
use tower::ServiceExt;
use tracing::info;

async fn handle_request(
    supergraph: Arc<Mutex<router::BoxCloneService>>,
    event: Request,
) -> Result<impl IntoResponse, Error> {
    let body = event.body();
    let event_payload: apollo_router::graphql::Request =
        serde_json::from_slice(body).expect("invalid graphql request");
    info!("👉 Proxying request to router: {:?}", event_payload);
    println!("👉 Proxying request to router: {:?}", event_payload);

    let builder = supergraph::Request::fake_builder()
        .header(CONTENT_TYPE, "application/json")
        .query(event_payload.query.clone().unwrap_or("".to_string()))
        .variables(event_payload.clone().variables)
        .extensions(event_payload.clone().extensions);
    // TODO: Avoid this weird builder hack.
    let request = if let Some(operation_name) = event_payload.clone().operation_name {
        builder.operation_name(operation_name).build().unwrap()
    } else {
        builder.build().unwrap()
    };

    // Lock the supergraph for the brief time we are calling it. This will have no real-world
    // impact on performance because the Lambda only ever serves one request at a time.
    let response = {
        let mut s = supergraph.lock().unwrap();
        s.call(request.try_into().unwrap())
    };
    let mut r = response.await?;

    // We serialize the event to a generic JSON value, and let `impl IntoResponse` to
    // the rest of the work for us to shape it into a nice Lambda response.
    let resp: serde_json::Value =
        serde_json::from_slice(r.next_response().await.unwrap().unwrap().to_vec().as_slice())?;
    info!("👉 Deserialized Response: {:?}", resp);
    println!("👉 Deserialized Response: {:?}", resp);

    Ok(resp)
}

async fn handler() -> Result<(), Error> {
    let config_path = env::var("APOLLO_ROUTER_CONFIG_PATH").unwrap_or("./router.yaml".to_string());
    let schema_path =
        env::var("APOLLO_ROUTER_SUPERGRAPH_PATH").unwrap_or("./supergraph.graphql".to_string());
    let config = fs::read_to_string(config_path)?;
    let schema = fs::read_to_string(schema_path)?;
    let configuration = serde_yaml::from_str::<Configuration>(&config).unwrap();

    // We set up the supergraph during the initialization of the Lambda, and reuse
    // it across invocations. It needs to be a Mutex because we need to be able to
    // to mutate it from within the Lambda handler.
    let supergraph = Arc::new(Mutex::new(
        TestHarness::builder()
            .configuration(Arc::new(configuration))
            .schema(&schema)
            // Without this all subgraphs get an empty response by default.
            .with_subgraph_network_requests()
            .build_router()
            .await?,
    ));

    // Wait until the service is ready via ServiceExt::ready. This keeps us in the initialization
    // phase, which typically has more resources allocated to it.
    {
        let mut s = supergraph.lock().unwrap();
        s.ready().await?;
    }

    // Set up the Lambda event handler, and Arc clone our supergraph so we can safely
    // pass it across async boundaries.
    run(service_fn(|event: Request| async {
        let s = Arc::clone(&supergraph);
        handle_request(s, event).await
    }))
    .await
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
