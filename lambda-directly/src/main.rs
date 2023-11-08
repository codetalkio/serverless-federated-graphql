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

    // Before we cast the Router YAML configuration to a strongly typed struct, we need to
    // manually handle any environment variables that are used in the configuration. This is
    // normally handled by the regular Router, but is missing when manually loading the config.
    let mut untyped_config = serde_yaml::from_str::<serde_yaml::Value>(&config).unwrap();
    if let Some(map) = untyped_config.as_mapping_mut() {
        if let Some(serde_yaml::Value::Mapping(ref mut nested_map)) =
            map.get_mut(&serde_yaml::Value::from("override_subgraph_url"))
        {
            for (_key, nested_value) in nested_map.iter_mut() {
                // Remove any env. parts of the string, which is specific to the Apollo configuration
                // format (e.g. "${env.SUBGRAPH_USERS_URL:-http://127.0.0.1:3065/}").
                let subgraph_override =
                    serde_yaml::to_string(nested_value).unwrap().replace("${env.", "${");
                // Expand any environment variables, and fallbacks, using the shell environment.
                let expanded_env = shellexpand::env(&subgraph_override).unwrap();
                // Replace the current original value with the expanded value.
                *nested_value = serde_yaml::Value::from(expanded_env);
            }
        }
    }

    // We can finally convert our untyped YAML configuration into a strongly typed Configuration
    // struct.
    let configuration = serde_yaml::from_value::<Configuration>(untyped_config).unwrap();

    let body = event.body();
    // let event_payload = std::str::from_utf8(body).expect("invalid utf-8 sequence");
    let event_payload: apollo_router::graphql::Request =
        serde_json::from_slice(body).expect("invalid graphql request");
    info!("👉 Proxying request to router: {:?}", event_payload);
    println!("👉 Proxying request to router: {:?}", event_payload);

    let builder = supergraph::Request::fake_builder()
        .header(CONTENT_TYPE, "application/json")
        .query(event_payload.query.unwrap_or("".to_string()))
        .variables(event_payload.variables)
        .extensions(event_payload.extensions);
    // TODO: Avoid this weird hack.
    let request = if let Some(operation_name) = event_payload.operation_name {
        builder.operation_name(operation_name).build().unwrap()
    } else {
        builder.build().unwrap()
    };

    let supergraph = TestHarness::builder()
        .configuration(Arc::new(configuration))
        .schema(&schema)
        // Without this all subgraphs get an empty response by default.
        .with_subgraph_network_requests()
        .build_router()
        .await?;
    let mut response = supergraph.oneshot(request.try_into().unwrap()).await?;

    // Alternatively, deserialize to apollo_router::graphql::Response.
    let resp: serde_json::Value = serde_json::from_slice(
        response.next_response().await.unwrap().unwrap().to_vec().as_slice(),
    )?;
    info!("👉 Deserialized Response: {:?}", resp);
    println!("👉 Deserialized Response: {:?}", resp);

    Ok(resp)
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
