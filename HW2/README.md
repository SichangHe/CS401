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
and downloads the dataset.
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

The REST API server exposes a POST endpoint at `/api/recommend`, port 52004.
A request contains a list of songs:

```jsonc
{
    "songs": [
        "name", // …
    ]
}
```

The response contains song recommendations:

<!-- TODO: Update the implementation to sync. -->

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

- The HTTP server is implemented using [Axum](https://github.com/tokio-rs/axum),
    and serves the REST API.
    Per request, it requests the rule server for recommendation rules.
- The file watcher uses [`notify`](https://github.com/notify-rs/notify) to
    watch the *data directory* specified in environment variable `DATA_DIR` for
    file events, and notifies the rule server when events occur.
    It also implements retry logic in case that `notify` fails.
- The rule server reads the *rules file* and stores the rules in memory.
    Upon events from the file watcher,
    the rule server checks the *checkpoint file* to verify that
    the generation time is newer than the recorded one,
    and reads the rules from the *rules file* if updated.

I used a [GenServer](https://hexdocs.pm/elixir/GenServer.html)-like actor
model to manage interactions between the three parties so all the interactions
are fully asynchronous and non-blocking.
The actor library is extracted into
[the `tokio_gen_server` crate](https://crates.io/crates/tokio_gen_server).

### 3. REST API Client

Generate requests using songs in `songs.csv`. Request with arbitrary number of songs taken from the user.

Web-based front-end.

## Part 2: Continuous Integration and Continuous Delivery

### 1. Create Docker Containers

- ML container: generate recommendation rules and save them.
- Front-end container: read ML model, run REST server.
    - Export port from Docker.

The container configuration in `compose.yml` and `Dockerfile` is bootstrapped with:

```sh
docker init
```

The build process and the final containers are separated to reduce container sizes. A Rust container image is used to build both the ML processor and the REST server. Its artifacts are then copied to the final containers.

To build locally:

```sh
DOCKER_DEFAULT_PLATFORM="linux/amd64" docker compose build
```

- Publish on DockerHub.

### 2. Configure the Kubernetes deployment and service

Specify a Kubernetes deployment and a service.

#### Sharing the Model over a Persistent Volume

Write a persistent volume claim to mount `project2-pv-sh623` in `/home/sh623/project2-pv`.

The web server needs to watch the model file in the persistent volume for changes.

### 3. Configure Automatic Deployment in ArgoCD

Configure ArgoCD to watch the GitHub repository for changes.

Pass in the pointer to the dataset to the ML container.

#### Automatically Updating Rules when the Dataset Changes

Change pod name on update, or other methods.

Other method: use jobs and set time-to-live after finish so they self-destruct.

## Part 3: Exercise and Evaluate Continuous Delivery

Test that ArgoCD redeploys when we update

- the Kubernetes deployment,
- the code,
- the training dataset.

Measure how long the CI/CD pipeline takes to update the deployment by continuously issuing requests using your client and checking for the update in the server's responses (either the version or dataset date).

Estimate if and for how long your application stays offline.
