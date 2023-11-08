use apollo_router::{ApolloRouterError, Configuration, RouterHttpServer};
use lambda_http::{run, service_fn, Error, IntoResponse, Request};
use reqwest::{header::CONTENT_TYPE, Client, Response};
use std::env;
use std::fs;
use tracing::info;

/// Invoke the router locally by sending the event to the router's local HTTP server.
async fn invoke(event: &Request) -> Result<Response, Error> {
    let url = format!("http://127.0.0.1:4000");

    let body = event.body();
    let event_payload = std::str::from_utf8(body).expect("invalid utf-8 sequence");

    let client = Client::new();
    info!("Proxying request to router: {:?}", event_payload);

    let resp = client
        .post(url)
        // TODO: Pass down whitelisted headers from the Lambda event.
        .header(CONTENT_TYPE, "application/json")
        .body(event_payload.to_string())
        .send()
        .await?;
    info!("Response from router: {:?}", resp);
    Ok(resp)
}

/// Pass on the Lambda event to the router and return the response.
///
/// NOTE: We keep retrying the request every 10ms until we get a response from
/// the router. This is because the router takes a short time to start up.
async fn handle_request(event: Request) -> Result<impl IntoResponse, Error> {
    let mut retries = 0;
    let mut response = invoke(&event).await;
    while retries < 500 && response.is_err() {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        response = invoke(&event).await;
        retries += 1;
    }
    println!("Retries: {}, waited a total {}ms", retries, retries * 10);
    let resp = response?;
    let status = resp.status();
    let payload = resp.json::<serde_json::Value>().await?;

    // TODO: Return whitelisted headers from the Router response.
    Ok((status, payload))
}

async fn start_router(schema: String, configuration: Configuration) -> Result<(), Error> {
    RouterHttpServer::builder()
        .configuration(configuration)
        .schema(schema)
        .start()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

async fn handler() -> Result<(), Error> {
    // Load configurations during the init phase of the Lambda.
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

    // Start a local Apollo Router server.
    tokio::spawn(async move { start_router(schema, configuration).await });

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
