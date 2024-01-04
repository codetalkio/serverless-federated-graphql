# Cosmo Router

This variant constructs a Go program that uses the Cosmo Router as a library, and wraps it up in Go calling code for AWS Lambda.

Usage:

Install dependencies:

```bash
$ go mod tidy
```

Build the go module for Arm64 and without RPC (see why for the [RPC part here](https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-from-the-go1-x-runtime-to-the-custom-runtime-on-amazon-linux-2/)):

```bash
$ GOOS=linux GOARCH=arm64 go build -tags lambda.norpc -o bin/bootstrap cmd/main.go
```

Put your configuration files in the same directory as the binary:

```bash
.
├── ms-cosmo
│   ├── bootstrap
│   ├── cosmo.yaml
│   └── supergraph.json
```

And deploy it to AWS Lambda using your favorite method, onto a runtime of `provided.al2023`.

I also recommend setting the following environment variables, to disable any additional behaviors on start up and to properly pick up configuration:

- CONFIG_PATH: 'cosmo.yaml'
- ROUTER_CONFIG_PATH: 'supergraph.json'
- ENGINE_ENABLE_REQUEST_TRACING: 'false'

I've also added a module that lets you overwrite your subgraph URLs via environment variables. E.g. if you have the subgraph `products` and want to overwrite its URL, you can set the env variable `SUBGRAPH_PRODUCTS_URL` to the new URL.

## Development

Run the router in local HTTP mode:

```bash
$ HTTP_PORT=4010 go run cmd/main.go
```

Or,

```bash
$ HTTP_PORT=4010 bunx nodemon --watch './**/*.go' --signal SIGTERM --exec 'go' run cmd/main.go
```

Make a simple request that doesn't require any running subgraphs:

```bash
$ curl --data '{ "query": "query ExampleQuery{ products { id } }", "operationName": "ExampleQuery" }'  --header 'Content-Type: application/json' http://localhost:4010
```

Update dependencies:

```bash
$ go mod tidy
```
