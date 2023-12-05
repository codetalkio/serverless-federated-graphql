package main

import (
	// Cosmo Router.
	routercmd "github.com/wundergraph/cosmo/router/cmd"
	// AWS Lambda.
	"fmt"

	"github.com/aws/aws-lambda-go/lambda"
	// Proxy.

	"net/http"
	"time"

	// Local server.
	"io"
	"log"
	"os"

	"github.com/awslabs/aws-lambda-go-api-proxy/httpadapter"
)

func invoke(r *http.Request) (*http.Response, error) {
	url := "http://127.0.0.1:4000/graphql"

	req_body, err := io.ReadAll(r.Body)
	if err != nil {
		return nil, err
	}

	log.Printf("Proxying request to router: %s\n", req_body)

	req, err := http.NewRequest("POST", url, r.Body)
	if err != nil {
		return nil, err
	}
	req.Header.Add("Content-Type", "application/json")

	client := &http.Client{}
	res, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()
	return res, nil
}

func HandleRequest(w http.ResponseWriter, r *http.Request) {
	var retries int
	retries = 0

	var res *http.Response
	var err error
	res, err = invoke(r)
	for {
		if retries >= 500 || err == nil {
			break
		}
		// Sleep for 10 ms.
		// This is to give the router time to start up.
		time.Sleep(time.Duration(time.Duration(retries).Milliseconds()))
		res, err = invoke(r)
	}
	log.Printf("Retries: %d, waited a total %dms\n", retries, retries*10)

	if err != nil {
		http.Error(w, "Error handling request", http.StatusInternalServerError)
		return
	}
	log.Printf("Response from router: %s\n", res.Body)

	body_res, err := io.ReadAll(res.Body)
	io.WriteString(w, string(body_res))
}

func main() {
	// Start the Router in the background.
	go routercmd.Main()

	http.HandleFunc("/", HandleRequest)

	// Determine if we're running in Lambda or locally.
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
