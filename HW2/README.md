# Project 2: DevOps and Cloud Computing

## Introduction

## Dataset

The dataset sample is available on the cluster at `/home/datasets/spotify-sample/`.

- `playlists-sample-ds1.csv` and `playlists-sample-ds2.csv`: 500 playlist each.
    - `playlists-sample-ds2.csv`: used to update the model.
- `song.csv` songs in the playlist.

<!-- The dataset sample is available on the cluster at `/home/datasets/spotify/`.

- `2023_spotify_ds1.csv` and `2023_spotify_ds2.csv`: 500 playlist each.
    - `2023_spotify_ds2.csv`: used to update the model.
- `2023_spotify_songs.csv` songs in the playlist. -->

## Part 1: Software Components

### 1. Playlist Rules Generator

Frequent Itemset Mining.

Takes pointer to the dataset from input.

Uses a checkpoint file `ml_processor_checkpoint.txt` to store:

```
<ML processor version> <dataset URL used> <nanoseconds of last generation since UNIX epoch>
```

### 2. REST API Server

POST endpoint at `/api/recommend`, port 52004.
A request contains a list of songs.

```jsonc
{
    "songs": [
        "name", // …
    ]
}
```

The response contains playlist recommendations.

```jsonc
{
    "playlist_ids": [
        "jfwioefjwoiefwjo",
    ], // playlist IDs that the user may enjoy
    "version": "vx.x.x", // version of the code running
    "model_date": "yyyy-mm-dd" // date when ML model was last updated
}
```

### 3. REST API Client

Generate requests using songs in `songs.csv`. Request with arbitrary number of songs taken from the user.

Web-based front-end.

## Part 2: Continuous Integration and Continuous Delivery

### 1. Create Docker Containers

- ML container: generate recommendation rules and save them.
- Front-end container: read ML model, run REST server.
    - Export port from Docker.

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
