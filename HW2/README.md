# Project 2: DevOps and Cloud Computing

## Introduction

## Dataset

The datasets used are hosted [here](https://homepages.dcc.ufmg.br/~cunha/hosted/cloudcomp-2023s2-datasets/).

Each dataset contains 500 playlists that contain songs.

<!-- The dataset sample is available on the cluster at `/home/datasets/spotify-sample/`.

- `playlists-sample-ds1.csv` and `playlists-sample-ds2.csv`: 500 playlist each.
    - `playlists-sample-ds2.csv`: used to update the model.
- `song.csv` songs in the playlist. -->

<!-- The dataset sample is available on the cluster at `/home/datasets/spotify/`.

- `2023_spotify_ds1.csv` and `2023_spotify_ds2.csv`: 500 playlist each.
    - `2023_spotify_ds2.csv`: used to update the model.
- `2023_spotify_songs.csv` songs in the playlist. -->

## Part 1: Software Components

### 1. Playlist Rules Generator

The ML Processor module runs a Frequent Itemset Mining algorithm to generate
rules for song recommendations.

The ML Processor uses the *data directory* specified in `DATA_DIR` to store
both the dataset and generated artifacts.
It takes an URL to the dataset from environment variable `DATASET_URL`,
and downloads the dataset using [`aria2c`](https://aria2.github.io/).
It then uses [the Aprirori algorithm](https://en.wikipedia.org/wiki/Apriori_algorithm)
in [this Rust implementation found on GitHub](https://github.com/remykarem/apriori-rs)
to generate the recommendation rules.
The rules are encoded using [`bincode`](https://github.com/bincode-org/bincode),
and saved to the *rules file* named `rules.bincode` in the *data directory*.

To avoid regenerating the same rules every time the ML Processor is run,
after generating the rules,
it records a *checkpoint file* named `ml_processor_checkpoint.txt` that contains:

```xml
<ML processor version> <dataset URL used> <generation time in nanoseconds since UNIX epoch>
```

When the ML Processor is run,
it first checks the *checkpoint file* to see if the current rules already are
generated using the same ML Processor version and the same dataset.
If not, it proceeds to generate the rules.
The generation time is for the REST API Server to know when the rules were
last updated.

### 2. REST API Server

The REST API Server exposes a POST endpoint at `/api/recommend`, port 52004.
A request contains a list of songs:

```jsonc
{
    "songs": [
        "name", // …
    ]
}
```

The response contains song recommendations:

```jsonc
{
    "songs": [
        "jfwioefjwoiefwjo", // …
    ],
    "version": "x.x.x", // version of the code running
    "model_date": "YYYY-MM-dd HH:mm:ss.SSSSSS" // date when recommendation rules were last updated
}
```

The server is implemented in three parts.

- The *HTTP server* is implemented using [Axum](https://github.com/tokio-rs/axum),
    and serves the REST API on the port specified in environment variable `PORT`.
    Per request, it requests the *rule server* for recommendation rules.
- The *file watcher* uses [`notify`](https://github.com/notify-rs/notify) to
    watch the *data directory* specified in environment variable `DATA_DIR` for
    file events, and notifies the *rule server* when events occur.
    It also implements retry logic in case that `notify` fails.
- The *rule server* reads the *rules file* and stores the rules in memory.
    Upon events from the *file watcher*,
    the *rule server* checks the *checkpoint file* to verify that
    the generation time is newer than the recorded one,
    and reads the rules from the *rules file* if updated.

I use a [GenServer](https://hexdocs.pm/elixir/GenServer.html)-like actor
model to manage interactions between the three parties so all the interactions
are fully asynchronous, non-blocking, and retry-native.
The actor library is extracted into
[the `tokio_gen_server` crate](https://crates.io/crates/tokio_gen_server).

### 3. REST API Client

The REST API client at `rest_client.py` can be used to request the REST API
server with arbitrary number of songs taken from the user.

```sh
$ python3 rest_client.py --help
Usage: rest_client.py [<FLAG>] <Song recommender server IP> <Song recommender server port> <Song 0> [<Song 1> … <Song n>]
<FLAG>:
    -h, --help: Print this help message
    -c, --continuous: Make continuous requests to the server and measure response changes
```

For example, we can generate a request using these songs:

```sh
$ python3 rest_client.py 10.110.141.13 52004 DNA. HUMBLE. Magnolia "Slippery (feat. Gucci Mane)" "I Get The Bag (feat. Migos)"
Recommendation server v0.2.0 with model from 2024-02-16 06:27:06.328215627.
Song recommendations:
    Tunnel Vision
    Butterfly Effect
    T-Shirt
    goosebumps
    Mask Off
    XO TOUR Llif3
    Slippery (feat. Gucci Mane)
    Bank Account
```

The "continuous" flag can be used to track deployment changes as used later in
Part 3.

## Part 2: Continuous Integration and Continuous Delivery

### 1. Create Docker Containers

I generate two containers:

- ML container: runs the ML Processor generate recommendation rules and save them.
- Frontend container: runs the REST API Server,
    which reads rules generated by the ML Processor and serves HTTP requests.
    - Export port 3000 from Docker.

The container configuration in `compose.yml` and `Dockerfile` is bootstrapped with:

```sh
docker init
```

To reduce container sizes,
the build process and the final containers are separated .
A Rust container image is used to build both
the ML processor and the REST server.
Its artifacts are then copied to the final containers.

To build locally:

```sh
DOCKER_DEFAULT_PLATFORM="linux/amd64" docker compose build
```

The images are published on DockerHub as `sssstevenhe/cs401-hw2-ml-processor`
and `sssstevenhe/cs401-hw2-rest-server`.

### 2. Configure the Kubernetes deployment and service

The Kubernetes configurations are defined in `k8s-tasks.yml`, including:

- A persistent volume claim to mount `project2-pv-sh623` in
    `/home/sh623/project2-pv`.
- A job that runs the ML container with the persistent volume mounted at
    `/ml-data` so the dataset, *checkpoint file*, and *rules file* are
    all saved and shared.
- A deployment to run the Frontend container with the persistent volume
    mounted at `/ml-data` and port set to 3000.
- A service that exposes the Frontend container on port 52004.

I chose to use a job to run the ML container instead of a deployment or pod,
because it is a one-off task to be done at the beginning of each deployment.
There are no problems that it is run every time the configuration is updated
because the ML Processor checks the *checkpoint file* to
skip rule generation when the configuration is unchanged.
I set a time-to-live-after-finished on this job,
so it gets deleted 60sec after it finishes.
This gets around the problem of the immutable job variables.
The downside to this approach is that the minimum update interval is 60sec,
and that when the ML container fails, it needs to be deleted manually.

### 3. Configure Automatic Deployment in ArgoCD

I configured ArgoCD to watch my GitHub repository for changes.
The configuration is generated by the ArgoCD web UI, and attached below.

```yml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: cs401-sh623-hw2
  finalizers:
    - resources-finalizer.argocd.argoproj.io
spec:
  destination:
    name: ''
    namespace: sh623
    server: 'https://kubernetes.default.svc'
  source:
    path: HW2/
    repoURL: 'https://github.com/SichangHe/CS401.git'
    targetRevision: HEAD
    directory:
      jsonnet:
        tlas:
          - {}
  sources: []
  project: sh623-project
  syncPolicy:
    automated:
      prune: true
    syncOptions: []
```

All deployment configurations are defined in the `k8s-tasks.yml` file,
including the pointer to the dataset for the ML container.
Therefore, any changes are pushed into production by changing the content of
`k8s-tasks.yml`.

#### Automatically Updating Rules when the Dataset Changes

Instead of changing pod name on update to work around the problem of immutable
job variables,
I use jobs for the ML containers and set time-to-live-after-finished so they
self-destruct, as detailed above.

Initially,
this caused the ML container jobs to be spawned in a loop because I enabled Argo
CD self-healing in its synchronization policy. In details,
the ML container is deleted 60sec after it finishes,
which Argo CD detects as an out-of-sync to the manifest and creates a new ML
container job.
I disabled self-healing in the Argo CD synchronization policy to fix this.

## Part 3: Exercise and Evaluate Continuous Delivery

<!-- TODO:
Test that ArgoCD redeploys when we update

- the training dataset.

Measure how long the CI/CD pipeline takes to update the deployment by continuously issuing requests using your client and checking for the update in the server's responses (either the version or dataset date).

Estimate if and for how long your application stays offline.
-->

### Updating the Kubernetes Deployment

First,
we test changing the Kubernetes deployment by changing the replica count from 3
to 2.
I started the measurement client in continuous mode on the VM (the results are
copied to `measurement-update-k8s.csv`):

```sh
python3 rest_client.py -c 10.110.141.13 52004 DNA.
```

I then pushed the changes to the Git remote from my machine (UTC+8)
and immediately recorded the system time:

```sh
$ git push && date +"%T.%3N"
# Git output…
   3b49044..01e6084  main -> main
15:09:19.197
```

The measurement client makes one request to the server every second and prints
in a CSV format:

```csv
<Timestamp when sending request>,<Response time (ms)>,<Version>,<Model date>
```

For example, below are some of the rows in `measurement-update-k8s.csv`:

```csv
02:09:09,17.449,0.2.0,2024-02-16 06:27:06.328215627
02:09:12,19.813,0.2.0,2024-02-16 06:27:06.328215627
02:13:00,16.308,0.2.0,2024-02-16 06:27:06.328215627
02:13:01,12.724,0.2.0,2024-02-16 06:27:06.328215627
02:13:08,16.381,0.2.0,2024-02-16 06:27:06.328215627
```

Note that the time zone on the VM is UTC,
and the response time includes the time spent running cURL.

If the data of one request are similar to the previous according to some
heuristics, the client omits printing them.

#### Results Updating The Kubernetes Deployment

The application stayed online.
There was no observable change in the server's response time after the
deployment. As I recorded in `measurement-update-k8s.csv`,
the response time stayed around 20ms.

The CD pipeline took around 3min 42sec to update the deployment.
At "Sat Feb 17 2024 15:13:01 GMT+0800",
Argo CD detected the change in the Git repository and updated the deployment,
as shown in its web UI. This is largely due to Argo CD's default sync interval
of 3min.

### Updating The Code

I first bumped `rest_server`'s version from `0.2.0` to `0.2.1`,
and bumped the versions in the Docker compose file accordingly.
I then rebuilt the containers and pushed the new image to Docker Hub.

Similar to the previous experiment,
I pushed the changes to the Git remote and immediately recorded the system time:

```sh
$ git push && date +"%T.%3N"
# Git output…
   5a63e30..4c070d6  main -> main
15:46:11.625
```

Later, I started the measurement client in continuous mode on the VM and caught the service going down at the exact right moment.

```csv
02:48:58,11.968,0.2.0,2024-02-16 06:27:06.328215627
02:49:04,12.778,0.2.0,2024-02-16 06:27:06.328215627
02:49:06,913.243,,
02:49:09,12.914,0.2.1,2024-02-16 06:27:06.328215627
02:49:11,914.123,,
02:49:12,187.220,0.2.1,2024-02-16 06:27:06.328215627
02:49:12,12.424,0.2.1,2024-02-16 06:27:06.328215627
02:49:14,17.210,0.2.1,2024-02-16 06:27:06.328215627
```

As Argo CD shows in its web UI,
it updated the deployment at "Sat Feb 17 2024 15:49:01 GMT+0800".
From 49:06 to 49:08 (for 3sec) and at 49:11,
the measurement client had its requests timed out.

#### Evaluating The Code Update

Inspecting the deployment logs shows that the two servers were starting up and
reading the rules from the *rules file*, so they deferred the response.

```rust
$ kubectl logs cs401-sh623-hw2-deployment-cb8c49f45-br8rr
2024-02-17T07:49:04.050429Z  INFO run{port="3000" data_dir="/ml-data"}:serve{port="3000"}: rest_server::serve: Starting server.
2024-02-17T07:49:04.050608Z  INFO handle_cast: rest_server::read_rules: Reading rules. when=Instant { tv_sec: 765115, tv_nsec: 572654102 }
2024-02-17T07:49:04.050683Z  INFO handle_cast: rest_server::watch_file: Initializing file watcher.
2024-02-17T07:49:05.952543Z  INFO rest_server::serve: request=RecommendationRequest { songs: ["DNA."] }
2024-02-17T07:49:05.952761Z  WARN handle_call: rest_server::read_rules: Deferring query reply.
// …
2024-02-17T07:49:08.957120Z  WARN handle_call: rest_server::read_rules: Deferring query reply.
2024-02-17T07:49:09.590256Z  INFO make_rules_map{rules_path="/ml-data/rules.bincode"}: rest_server::read_rules: Read rules from file. n_rules=341941
2024-02-17T07:49:09.888482Z  INFO handle_cast: rest_server::read_rules: New rules. new_datetime="2024-02-16 06:27:06.328215627"
2024-02-17T07:49:09.950573Z  INFO rest_server::serve: request=RecommendationRequest { songs: ["DNA."] }
// …
```

```rust
$ kubectl logs cs401-sh623-hw2-deployment-cb8c49f45-w5rkd
2024-02-17T07:49:05.597039Z  INFO run{port="3000" data_dir="/ml-data"}:serve{port="3000"}: rest_server::serve: Starting server.
2024-02-17T07:49:05.597595Z  INFO handle_cast: rest_server::read_rules: Reading rules. when=Instant { tv_sec: 765117, tv_nsec: 119735223 }
2024-02-17T07:49:05.599042Z  INFO handle_cast: rest_server::watch_file: Initializing file watcher.
2024-02-17T07:49:06.951733Z  INFO rest_server::serve: request=RecommendationRequest { songs: ["DNA."] }
2024-02-17T07:49:06.951927Z  WARN handle_call: rest_server::read_rules: Deferring query reply.
// …
2024-02-17T07:49:09.957802Z  WARN handle_call: rest_server::read_rules: Deferring query reply.
2024-02-17T07:49:10.804069Z  INFO make_rules_map{rules_path="/ml-data/rules.bincode"}: rest_server::read_rules: Read rules from file. n_rules=341941
// …
2024-02-17T07:49:11.101222Z  INFO handle_cast: rest_server::read_rules: New rules. new_datetime="2024-02-16 06:27:06.328215627"
2024-02-17T07:49:12.950657Z  INFO rest_server::serve: request=RecommendationRequest { songs: ["DNA."] }
// …
```

Realistically,
the deferred requests would have been reevaluated every second and eventually
served when the servers finished reading the rules,
so the clients would instead experience a slow response.
Argo CD started the new deployment at 49:01,
and it took 3sec for Kubernetes to launch the new containers and start routing
new traffic to them. The first server was reading the rules from 49:04 to 49:09,
and the second server from 49:04 to 49:11.
Kubernetes started to redirect requests to the new servers at 49:06,
so some requests might take up to 5sec to complete.

Therefore, it took 3min 0sec for the pipeline to finish updating the deployment.
The application was never offline, but had a slow response period for 5sec.

Ideally, Kubernetes would have waited for the new servers to be ready before
routing traffic to them. This would be a future enhancement.

### Updating The Training Dataset

First,
I updated the `DATASET_URL` environment variable for the ML container in
`k8s-tasks.yml` to point to the second dataset rather than the first.
I pushed this change to the Git remote and immediately recorded the system time:

```sh
$ git push && date +"%T.%3N"
# Git output…
```
