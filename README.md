# Lambda Variants of Apollo Router

> Minimal/scrappy build of Apollo Router built for Lambda via Amazon Linux 2.

**TL;DR**: The `lambda-directly-optimized` beats all alternatives for Cold and Warm Starts (use the `bootstrap-directly-optimized-graviton-arm-size` binary).

Currently [Apollo Router](https://github.com/apollographql/router) does not support running in AWS Lambda (https://github.com/apollographql/router/issues/364). Instead it's focusing on running as a long-lived process, which means that it's less optimized for quick startup, as well as built with dependencies that does not mesh with Lambda's Amazon Linux 2 environment.

But what if we were a little bit creative? Could we get it to work? The answer is: Yes! (sorta)...

This repository contains two examples:

- `lambda-with-server/`: Spins up a Apollo Router using the [apollo-router crate](), and proxies Lambda Events to the HTTP server locally.
- `lambda-directly/`: Uses the [TestHarness](https://github.com/apollographql/router/blob/a6f129cdb75038eae6437e24876723194aeaf165/apollo-router/src/test_harness.rs#L38-L78) that Apollo Router uses to easily make GraphQL requests in its tests without needing a full Router. The Lambda takes the incoming event, runs it through the `TestHarness` and returns the result.
- `lambda-directly-optimized/`: Same approach as `lambda-directly`, but we only construct the [TestHarness](https://github.com/codetalkio/apollo-router-lambda/blob/a0899105794b50a7c9ab200131a8b45266328e96/lambda-directly-optimized/src/main.rs#L85-L91) once and then reuse it across all invocations. We also optimize loading configurations as well as initializing the Supergraph by doing it during [Lambda's Initialization phase](https://hichaelmart.medium.com/shave-99-93-off-your-lambda-bill-with-this-one-weird-trick-33c0acebb2ea), which runs at full resource. Additionally, we buid this for the ARM architecture and also optimize it for the AWS Graviton CPU.

We do some additional tricks to reduce the size of the `bootstrap-directly-optimized-graviton-arm-size` binary, which has an impact on Cold Starts:
- We [remove location details](https://github.com/johnthagen/min-sized-rust#remove-location-details), [panic string formatting](https://github.com/johnthagen/min-sized-rust#remove-panic-string-formatting-with-panic_immediate_abort), and [abort on panic](https://github.com/johnthagen/min-sized-rust#abort-on-panic)
- We [rebuild and optimize libstd](https://github.com/johnthagen/min-sized-rust#optimize-libstd-with-build-std) with build-std, which combined with the above brings us from ~71MB down to ~49MB.
- ~~We use [upx](https://github.com/upx/upx) to reduce the size of the binaries.~~ Unfortuntately, the overhead of decompressing the binary significantly increases Cold Start times, e.g. `lambda-directly-optimized` goes up from 0.8s to 2.5s, despite a binary reduction from 73.71MB to 18MB.

Check out the code and `Dockerfile` for each. There's really not a lot going on, and it is a minimal implementation compared to what you'd want in Production. My current recommendation would be either use the `bootstrap-directly-optimized-graviton-arm` binary produced from the `lambda-directly-optimized` approach in AWS Lambda, or to run Apollo Router in App Runner, which it does extremely well (I can max out the allowed 200 concurrent requests on a 0.25 CPU and 0.5GB Memory setting).

| Approach | Advantage     | Performance |
|----------| ------------- |-------------|
| `lambda-with-server` | · Full router functionality (almost) | · Cold Start: ~1.58s <br/>· Warm Start: ~49ms |
| `lambda-directly` | · No need to wait for a server to start first (lower overhead) | · Cold Start: ~1.32s <br/>· Warm Start: ~314ms |
| `lambda-directly-optimized` | · No need to wait for a server to start first (lower overhead)<br/>· Built for ARM<br/>· Optimized for the Graviton CPU | Optimized for size<br/>· Cold Start: ~0.7s <br/>· Warm Start: ~20ms<br/>Optimized for speed<br/>· Cold Start: ~0.9s <br/>· Warm Start: ~20ms |

Comparison to alternatives:

| Approach | Performance |
|----------| -------------|
| `Apollo Gateway`  | · Cold Start: ~1.23s <br/>· Warm Start: ~120ms |
| `GraphQL Mesh` | · Cold Start: ~0.9s <br/>· Warm Start: ~100ms |

Overview:

- [How to use](#how-to-use)
- [Cold Starts](#cold-starts)
- [Warm Starts](#warm-starts)
- [Comparison: Rust Subgraph in AWS Lambda](#comparison-rust-subgraph-in-aws-lambda)
- [Comparison: Federation via Apollo Gateway](#comparison-federation-via-apollo-gateway)
- [Comparison: Federation via GraphQL Mesh](#comparison-federation-via-graphql-mesh)

# How to use

Each of the approach are generic and can be used as-is. You can simply grab whichever variant you want from the [Releases](https://github.com/codetalkio/apollo-router-lambda/releases) page, which uploads the `bootstrap` artifact from each of them.

For example, let's say we want to try running `lambda-with-server`.

First we create a folder to hold our artifacts in, which we will .zip up and deploy to Lambda:

```bash
$ mkdir apollo-router
```

Then we download the relevant

```bash
$ curl -sSL https://github.com/codetalkio/apollo-router-lambda/releases/latest/download/bootstrap-directly-optimized-graviton-arm-size -o bootstrap
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
The `lambda-directly-optimized` approach is the only one that enters the realm of "acceptable" cold starts. Still high, but almost always below 1 second. Both of the other approachs unfortunately have quite a high cold start time. The `lambda-directly` approach wins by a tiny margin, but none are great. None of the variants talk to any Subgraphs, this is purely measuring the overhead of startup.

`lambda-with-server`

<img width="1635" alt="Direct Router Cold Screenshot 2023-10-31 at 20 50 17" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/5a55a92d-5f38-45b9-8b41-47c77bc6cc20">

A good 450ms of this is spent just waiting for the Router to spin up:

<img width="1377" alt="Apollo as Server in Lambda Screenshot 2023-10-30 at 23 19 50" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/e7167483-96a1-48b3-99c3-0df5748f1850">

Breadown of only the router (making no queries to subgraphs):

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 8.3 ms | 8.7 ms | 7.6 ms | 7.6 ms | 8 ms |
| Average cold start response time | 2870.9 ms | 2570.4 ms | 2174.1 ms | 1012.8 ms | 943.4 ms |
| Fastest warm response time | 6 ms | 6 ms | 6 ms | 6.9 ms | 6.9 ms |
| Slowest warm response time | 16.9 ms | 16.9 ms | 16.9 ms | 16.9 ms | 16.9 ms |
| Fastest cold response time  | 837 ms | 837 ms | 837 ms | 837 ms | 837 ms |
| Slowest cold response time | 3861.9 ms | 3861.9 ms | 2612.9 ms | 1625 ms | 1139 ms |


`lambda-directly`

<img width="1633" alt="Lambda Router Cold Screenshot 2023-10-31 at 20 57 47" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/cd3f4e41-91ef-41e2-ba1a-1213803bff30">

Breadown of only the router (making no queries to subgraphs):

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 10.1 ms | 8 ms | 8.7 ms | 8.9 ms | 7.6 ms |
| Average cold start response time | 720 ms | 505 ms | 511.5 ms | 455.2 ms | 424.9 ms |
| Fastest warm response time | 4.9 ms | 4.9 ms | 4.9 ms | 4.9 ms | 5.9 ms |
| Slowest warm response time | 29 ms | 29 ms | 29 ms | 29 ms | 11.9 ms |
| Fastest cold response time  | 373.9 ms | 373.9 ms | 373.9 ms | 373.9 ms | 375 ms |
| Slowest cold response time | 1426 ms | 918.9 ms | 700.9 ms | 700.9 ms | 507.9 ms |

`lambda-directly-optimized` (optimized for speed)

<img width="1635" alt="Cold start (No Query) Screenshot 2023-11-11 at 23 25 15" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/400b66fb-ada1-4031-bc39-7867e9e4a29f">

A few samples of `lambda-directly-optimized` (optimized for speed) Cold Starts:

<img width="1043" alt="Overview of Cold starts (No Query) Screenshot 2023-11-11 at 23 24 59" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/797b3342-4122-4092-81aa-58f58dc1bbdf">

Breadown of only the router (making no queries to subgraphs):

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 9.7 ms | 5.4 ms | 5.6 ms | 6.1 ms | 5.8 ms |
| Average cold start response time | 858 ms | 837.6 ms | 775.5 ms | 768.3 ms | 753.2 ms |
| Fastest warm response time | 4.9 ms | 4.9 ms | 4.9 ms | 4.9 ms | 4.9 ms |
| Slowest warm response time | 23 ms | 8 ms | 7 ms | 7 ms | 7 ms |
| Fastest cold response time  | 719 ms | 719 ms | 719 ms | 719 ms | 719 ms |
| Slowest cold response time | 1075 ms | 981.9 ms | 981.9 ms | 981.9 ms | 868 ms |

`lambda-directly-optimized` (optimized for size)

<img width="1422" alt="Cold start (No Query) Screenshot 2023-11-13 at 18 01 42" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/48bf0a0e-f841-49f9-aa87-7d67ee34e506">

A few samples of `lambda-directly-optimized` (optimized for size) Cold Starts:

<img width="1108" alt="Overview of Cold starts (No Query) Screenshot 2023-11-13 at 18 01 10" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/e8e97766-8920-4de7-afa9-cfdf5aacb7f2">

Breadown of only the router (making no queries to subgraphs):

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 5.2 ms | 5.6 ms | 5.2 ms | 5.6 ms | 5.5 ms |
| Average cold start response time | 735.8 ms | 735.6 ms | 698.1 ms | 698.8 ms | 688.1 ms |
| Fastest warm response time | 4 ms | 4 ms | 4.9 ms | 4.9 ms | 4.9 ms |
| Slowest warm response time | 72.9 ms | 20.9 ms | 9.9 ms | 8 ms | 8 ms |
| Fastest cold response time  | 617 ms | 617 ms | 617 ms | 617 ms | 617 ms |
| Slowest cold response time | 985 ms | 985 ms | 894.9 ms | 894.9 ms | 762 ms |


# Warm Starts

Here we see both `lambda-directly-optimized` and `lambda-with-server` shine. Once it's started the Apollo Router/TestHarness, then it has relatively little overhead. `lambda-directly` on the other hand will build a `TestHarness` on each new request, and will keep paying a high cost, slowing it down.

Both of these examples talk to 1 warm subgraph implemented in Rust, to simulate a real warm run.

`lambda-with-server`

<img width="1637" alt="Direct Router Warm (Products query)  Screenshot 2023-10-31 at 20 50 48" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/fd0ee045-1b1e-4417-a077-316ddbe8f35c">


`lambda-directly`

<img width="1637" alt="Lambda Router Warm (Products query) Screenshot 2023-10-31 at 20 41 43" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/b9b8e456-d32e-4c0a-b1e4-72cb7c9cbe9c">


`lambda-directly-optimized` (optimized for size)

<img width="1485" alt="Warm start (talking to Products) Screenshot 2023-11-11 at 23 48 46" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/9bfbe078-2c27-4739-a29a-6b3a20ff09ff">

Breadown of the router making a query to a subgraph:

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 8.1 ms | 7.2 ms | 8.1 ms | 5.8 ms | 6.9 ms |
| Average cold start response time | 767.7 ms | 700.3 ms | 695.5 ms | 660.8 ms | 673 ms |
| Fastest warm response time | 4.9 ms | 4.9 ms | 4.9 ms | 4.9 ms | 4.9 ms |
| Slowest warm response time | 9.9 ms | 17 ms | 17 ms | 17 ms | 17 ms |
| Fastest cold response time  | 604 ms | 604 ms | 604 ms | 604 ms | 609.9 ms |
| Slowest cold response time | 834.9 ms | 781 ms | 781 ms | 769 ms | 769 ms |



# Comparison: Rust Subgraph in AWS Lambda

For comparison so that you know how far we _could_ go, here's a subgraph in Rust implemented using [async-graphql](https://github.com/async-graphql/async-graphql) and wrapped up in [cargo-lambda](https://www.cargo-lambda.info/).

Cold Start (201ms):

<img width="1411" alt="Screenshot 2023-10-21 at 12 13 18" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/e561ce43-ffd3-4ef3-bb95-8d5619035f37">


Warm Start (8ms):

<img width="1411" alt="Screenshot 2023-10-21 at 12 14 41" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/f71beb7f-b210-46ec-80ea-fc0de86f9581">


# Comparison: Federation via Apollo Gateway

To have something to compare the Apollo Router PoC more directly against, here's one alternative using [Apollo Gateway](https://www.apollographql.com/docs/apollo-server/using-federation/apollo-gateway-setup).

Cold start (1.23ms):

<img width="1350" alt="Cold start ms-gateway Screenshot 2023-10-22 at 21 45 34" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/d8958d82-529a-4b63-98c9-db90b06f0fe2">


Warm start (120ms):

<img width="1412" alt="Warm start subgraph times Screenshot 2023-10-22 at 16 13 26" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/577d8e5b-afc6-4d2f-b22c-7b61f94a473d">

Breadown of only the router (making no queries to subgraphs):

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 6.9 ms | 6.9 ms | 7 ms | 6.5 ms | 6.8 ms |
| Average cold start response time | 1024.1 ms | 1006.5 ms | 970.9 ms | 934.4 ms | 868.6 ms |
| Fastest warm response time | 5 ms | 5 ms | 5 ms | 5 ms | 5 ms |
| Slowest warm response time | 13 ms | 13 ms | 13 ms | 13 ms | 13 ms |
| Fastest cold response time  | 840 ms | 840 ms | 840 ms | 840 ms | 840 ms |
| Slowest cold response time | 1269 ms | 1269 ms | 1121.9 ms | 1006 ms | 980.9 ms |

Breadown of the router making a query to a subgraph:

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 7.2 ms | 7.2 ms | 6.8 ms | 7.8 ms | 6.9 ms |
| Average cold start response time | 1068.4 ms | 1068.6 ms | 897.3 ms | 888.7 ms | 854 ms |
| Fastest warm response time | 4.9 ms | 4.9 ms | 4.9 ms | 4.9 ms | 5.9 ms |
| Slowest warm response time | 13.9 ms | 13.9 ms | 13.9 ms | 13.9 ms | 13.9 ms |
| Fastest cold response time  | 842 ms | 842 ms | 842 ms | 842 ms | 842 ms |
| Slowest cold response time | 1476 ms | 1274.9 ms | 1173 ms | 1003 ms | 903.9 ms |


# Comparison: Federation via GraphQL Mesh

Another comparison point against the Apollo Router PoC, here's one alternative using [GraphQL Mesh](https://the-guild.dev/graphql/mesh).

Cold start (956ms):

<img width="1352" alt="Cold start ms-mesh Screenshot 2023-10-22 at 21 42 45" src="https://github.com/codetalkio/apollo-router-lambda/assets/1189998/686ee42b-0371-420c-adbd-facbe640f155">

Breadown of only the router (making no queries to subgraphs):

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 7.1 ms | 7.1 ms | 7 ms | 7 ms | 7 ms |
| Average cold start response time | 572.2 ms | 526.4 ms | 513.6 ms | 512 ms | 485.9 ms |
| Fastest warm response time | 5 ms | 5 ms | 5.9 ms | 5.9 ms | 5.9 ms |
| Slowest warm response time | 74 ms | 74 ms | 74 ms | 37 ms | 22 ms |
| Fastest cold response time  | 461.9 ms | 461.9 ms | 461.9 ms | 461.9 ms | 474.9 ms |
| Slowest cold response time | 680.9 ms | 648 ms | 592 ms | 592 ms | 588 ms |

Breadown of the router making a query to a subgraph:

| Measurement (ms) | 128 MB | 256 MB | 512 MB | 1024 MB | 2048 MB |
|-------------|-------------|-------------|-------------|-------------|-------------|
| Average warm start response time | 8.5 ms | 6.7 ms | 6.5 ms | 9 ms | 5.7 ms |
| Average cold start response time | 517.4 ms | 514.5 ms | 509.2 ms | 516 ms | 500.8 ms |
| Fastest warm response time | 4.9 ms | 4.9 ms | 4.9 ms | 4.9 ms | 4.9 ms |
| Slowest warm response time | 189 ms | 173 ms | 46 ms | 46 ms | 8 ms |
| Fastest cold response time  | 470 ms | 470 ms | 470 ms | 470 ms | 473 ms |
| Slowest cold response time | 709 ms | 629.9 ms | 599.9 ms | 581 ms | 541.9 ms |
