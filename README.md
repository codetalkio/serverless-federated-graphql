# Lambda Variants of Apollo Router

> Minimal/scrappy build of Apollo Router built for Lambda via Amazon Linux 2.

Consider this repo more of a proof-of-concept. Currently [Apollo Router](https://github.com/apollographql/router) does not support running in AWS Lambda (https://github.com/apollographql/router/issues/364). Instead it's focusing on running as a long-lived process, which means that it's less optimized for quick startup, as well as built with dependencies that does not mesh with Lambda's Amazon Linux 2 environment.

But what if we were a little bit creative? Could we get it to work? The answer is: Yes! (sorta)...

This repository contains two examples:

- `lambda-with-server/`: Spins up a Apollo Router using the [apollo-router crate](), and proxies Lambda Events to the HTTP server locally.
- `lambda-directly/`: Uses the [TestHarness](https://github.com/apollographql/router/blob/a6f129cdb75038eae6437e24876723194aeaf165/apollo-router/src/test_harness.rs#L38-L78) that Apollo Router uses to easily make GraphQL requests in its tests without needing a full Router. The Lambda takes the incoming event, runs it through the `TestHarness` and returns the result.

Check out the code and `Dockerfile` for each. There's really not a lot going on, and it is a minimal implementation compared to what you'd want in Production. My current recommendation would be to run Apollo Router in App Runner, which is does extremely well (I can max out the allowed 200 concurrent requests on a 0.25 CPU and 0.5GB Memory setting).

| Approach | Advantage     | Performance |
|----------| ------------- |-------------|
| `lambda-with-server` | · Full router functionality (almost) | · Cold Start: ~1.58s <br/>· Warm Start: ~49ms |
| `lambda-directly` | · No need to wait for a server to start first (lower overhead) | · Cold Start: ~1.32s <br/>· Warm Start: ~314ms |

# How to use

Each of the approach are generic and can be used as-is. You can simply grab whichever variant you want from the [Releases](https://github.com/codetalkio/apollo-router-lambda/releases) page, which uploads the `bootstrap` artifact from each of them.

For example, let's say we want to try running `lambda-with-server`.

First we create a folder to hold our artifacts in, which we will .zip up and deploy to Lambda:

```bash
$ mkdir apollo-router
```

Then we download the relevant 

```bash
$ curl -sSL https://github.com/codetalkio/apollo-router-lambda/releases/latest/download/bootstrap-directly-x86-64 -o bootstrap
  mv bootstrap ./apollo-router/bootstrap
```

If you want to use `lambda-directly` instead, you can use this URL instead:

```bash
$ curl -sSL https://github.com/codetalkio/apollo-router-lambda/releases/latest/download/bootstrap-directly-x86-64 -o bootstrap
$ mv bootstrap ./apollo-router/bootstrap
```

Now we just need to add our `router.yaml` and `supergraph.graphql` since the services will look these up from their same folder during startup:

```bash
# From whereever your router.yaml is:
$ cp router.yaml ./apollo-router/router.yaml
# From whereever your supergraph.graphql is:
$ cp supergraph.graphql ./apollo-router/supergraph.graphql
```

You now have the following contents in your `apollo-router` folder:

```bash
.
├── ms-router
│   ├── bootstrap
│   ├── router.yaml
│   └── supergraph.graphql
```

And you're ready to deploy using your preferred method of AWS CDK/SAM/SLS/SST/CloudFormation/Terraform.

# Cold Starts
Both of the approachs unfortunately have quite a high cold start time. The `lambda-directly` approach wins by a tiny margin, but none are great.

`lambda-with-server`

<img width="1635" alt="Direct Router Cold (Products query) Screenshot 2023-10-31 at 20 50 17" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/5a55a92d-5f38-45b9-8b41-47c77bc6cc20">

`lambda-directly`

<img width="1633" alt="Lambda Router Cold (No query, Minimal) Screenshot 2023-10-31 at 20 57 47" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/cd3f4e41-91ef-41e2-ba1a-1213803bff30">

# Warm Starts

Here we see `lambda-with-server` shine. Once it's started the Apollo Router, then it has relatively little overhead. `lambda-directly` on the other hand will build a `TestHarness` on each new request, and will keep paying a high cost, slowing it down.

`lambda-with-server`

<img width="1637" alt="Direct Router Warm (Products query)  Screenshot 2023-10-31 at 20 50 48" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/fd0ee045-1b1e-4417-a077-316ddbe8f35c">

`lambda-directly`

<img width="1637" alt="Lambda Router Warm (Products query) Screenshot 2023-10-31 at 20 41 43" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/b9b8e456-d32e-4c0a-b1e4-72cb7c9cbe9c">



