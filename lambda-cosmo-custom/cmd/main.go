package main

import (
	// Cosmo Router.
	routercmd "github.com/wundergraph/cosmo/router/cmd"

	// AWS Lambda.
	"github.com/aws/aws-lambda-go/lambda"

	// Proxy and utilities.
	"bytes"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"time"

	// HTTP adapter used for local server setup.
	"github.com/awslabs/aws-lambda-go-api-proxy/httpadapter"

	// Custom modules.
	_ "router/module"
)

// The invoke function is used to invoke the Cosmo Router. It simply proxies
// the request to the router and returns the response.
func invoke(req_body []byte) (*http.Response, error) {
	// The URL our Cosmo Router will be available on.
	url := "http://127.0.0.1:4005/graphql"

	// Create a new request with the same body as the original.
	req, err := http.NewRequest("POST", url, bytes.NewBuffer(req_body))
	if err != nil {
		return nil, err
	}
	req.Header.Add("Content-Type", "application/json")

	// Make the actual request to the router.
	client := &http.Client{}
	res, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	return res, nil
}

// Handle invoking the router and retrying the same request until
// it succeeds or we exceed retires. The retrying is to handle
// failures when the router is starting up.
func handler(w http.ResponseWriter, r *http.Request) {
	req_body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	log.Printf("Proxying request to router: %s\n", req_body)

	// Do an initial invoke and see if it succeeds.
	retries := 0
	res, err := invoke(req_body)
	for {
		// If we've exceeded retries or the request succeeded, break.
		if retries >= 500 || err == nil {
			break
		}
		// Sleep for 10 ms and try again.
		time.Sleep(5 * time.Millisecond)
		res, err = invoke(req_body)
		retries += 1
	}
	log.Printf("Retries: %d, waited a total %dms\n", retries, retries*10)

	if err != nil {
		http.Error(w, "Error handling request", http.StatusInternalServerError)
		return
	}

	// Finally, return the response.
	body_res, err := io.ReadAll(res.Body)
	if err != nil || body_res == nil || string(body_res) == "" {
		http.Error(w, "Error reading router response", http.StatusInternalServerError)
		return
	}
	log.Printf("Response from router: %s\n", body_res)
	io.WriteString(w, string(body_res))
}

func main() {
	// Start the Router in the background.
	go routercmd.Main()

	// Set up our handler as the root HTTP handler.
	http.HandleFunc("/", handler)

	// Determine if we're running in Lambda or locally. If HTTP_PORT is set
	// we start a local HTTP server, otherwise we start the Lambda handler.
	httpPort := os.Getenv("HTTP_PORT")
	if httpPort == "" {
		log.Println("Starting Lambda Handler")
		lambda.Start(httpadapter.New(http.DefaultServeMux).ProxyWithContext)
	} else {
		log.Printf("Starting HTTP server on port %s\n", httpPort)
		formattedPort := fmt.Sprintf("127.0.0.1:%s", httpPort)
		log.Fatal(http.ListenAndServe(formattedPort, nil))
	}
}
