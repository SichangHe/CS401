# Project 3: Serverless Computing and Monitoring Dashboard

## Introduction

- [ ] TODO

## Task 1: Serverless Function and Runtime

The data is stored in Redis under the `metrics` key.

### Data Output (Computed Metric and Function Results)

My serverless function `function.py` computes two stateless metrics at each
point in time:
the percentage of outgoing traffic bytes under the key
`percentage_outgoing_bytes` and the percentage of memory caching content under
the key `percentage_memory_caching`.

My function also compute a moving average utilization of each CPU over the last
minute under the key `moving_average_cpu_percent-X` (for CPU `X`)
by taking the arithmetic mean of all the CPU utilization percentages within the
last minute as of the last CPU metrics.
This is achieved by storing each CPU's utilization percentages and the
timestamps they are recorded in a list under the key `cpu_percent-X` in
`context.env`.

These keys (`percentage_outgoing_bytes`, `percentage_memory_caching`,
and `moving_average_cpu_percent-X`, totaling $N_{\text{CPU}} + 2$ keys)
are returned from the `handle` function as a JSON-encodable dictionary.

### Integration with the Serverless Framework

I use the image `lucasmsp/serverless:redis` and the deployment files as
provided, without changes. To set specify the serverless function,
I create a ConfigMap in `pyfile-cm.yml`, with a single key named `pyfile`,
whose value is my serverless function source code.
I also create another ConfigMap in `outputkey-cm.yml` that contains a single key
named `REDIS_OUTPUT_KEY` corresponding to `sh623-proj3-output` to store my
output in Redis.

I deploy my serverless function by applying my ConfigMaps and the provided
deployment file to my Kubernetes namespace:

```sh
kubectl -n sh623 apply -f outputkey-cm.yml -f pyfile-cm.yml -f serverless-deployment-course.yaml
```

## Task 2: Monitoring Dashboard

To display the monitoring information computed by my serverless function,
I leverage the metrics section of Phoenix web framework's built-in live
dashboard to build `dashboard`. Specifically,
I poll the Redis server for the metrics every 2.5 seconds,
and [register the metrics as telemetry
events](https://hexdocs.pm/phoenix/telemetry.html#telemetry-events);
in the dashboard's metrics configuration,
I [listen to these
events](https://medium.com/@marcdel/adding-custom-metrics-to-a-phoenix-1-5-live-dashboard-1b21a8df5cf1).
The live dashboard then automatically displays the metrics in real-time as a
line chart.

The Redis server to poll can be specified in the environment variables
`REDIS_HOST` and `REDIS_PORT`,
and the output key can be specified in the environment variable
`REDIS_OUTPUT_KEY`.

- [ ] Package your dashboard in a Docker image, create a Kubernetes Deployment specification, and a Service specification.
    - [ ] Expose dashboard on port 53004.

I generated the `Dockerfile` using `mix phx.gen.release --docker`,
added a Docker compose file, and built the image at `dashboard` with:

```sh
DOCKER_DEFAULT_PLATFORM="linux/amd64" docker compose build
```

The image built is on DockerHub, named `sssstevenhe/cs401-hw3-dashboard`.

## Task 3: Serverless Runtime

build a container image to replace the runtime image provided by the instructors.

In the runtime provided for this assignment, data is periodically read from Redis and passed in as parameters to the function. When the function returns, results are stored on Redis, where they can be later be read by the dashboard.

### Required Extensions

Your runtime should allow users to specify the following parameters through ConfigMaps:

- [ ] Redis Input Key: Your runtime should allow the user to configure a different Redis key to be monitored by your runtime. (This is equivalent to setting the `REDIS_INPUT_KEY` in the default runtime.)
- [ ] Redis Monitoring Period: Your runtime should allow the user to configure the period, in seconds, of how frequently the Redis key above should be monitored. (The default runtime sets this to 5 seconds.)
- [ ] Complex Function Support: Your runtime should allow the user to pass in the location of a Zip file containing the function's code. This will allow users to conveniently run functions that are implemented using multiple Python modules (that is, multiple Python files). This functionality should be provided in addition to the `pyfile` ConfigMap described above.
- [ ] Function Handler: Your runtime should allow the user to specify what function should be called as the "entry point". The entry point function should still receive `input` and `context` as parameters as in the original runtime.
