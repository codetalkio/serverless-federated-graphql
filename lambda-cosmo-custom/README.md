# Cosmo Router

Run the router in local HTTP mode:

```bash
$ HTTP_PORT=4010 go run main.go
```

Or,

```bash
$ HTTP_PORT=4010 bunx nodemon --watch './**/*.go' --signal SIGTERM --exec 'go' run main.go
```

Make a simple request that doesn't require any running subgraphs:

```bash
$ curl --data '{ "query": "query ExampleQuery{ __typename }", "operationName": "ExampleQuery" }'  --header 'Content-Type: application/json' http://localhost:4010
```

Update dependencies:

```bash
$ go mod tidy
```

Build the go module for Arm64 and without RPC:

```bash
$ GOOS=linux GOARCH=arm64 go build -tags lambda.norpc -o bin/bootstrap cmd/main.go
```

You can see more about build recommendations here https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-from-the-go1-x-runtime-to-the-custom-runtime-on-amazon-linux-2/.
