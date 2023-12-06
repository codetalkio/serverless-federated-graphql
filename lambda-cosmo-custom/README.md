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

Build the go module:

```bash
$ GOOS=linux GOARCH=arm64 go build -o bin/handler cmd/main.go
```
