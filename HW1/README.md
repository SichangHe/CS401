# Project 1: Big Data Programming Paradigms

## Dataset Description

The dataset used is in HDFS at `hdfs://localhost:9000/datasets/spotify/`.
`playlists.json` containing playlist metadata while `tracks.json`
containing information about songs present in the playlists.

## Tasks

### 1. Statistics about songs duration

Interquartile Range Rule
([IQRR](https://www.thoughtco.com/what-is-the-interquartile-range-rule-3126244)),
a technique that removes points outside an interval defined by the 1st
and 3rd quartiles.

- [ ] Generate a table containing
    - [ ] the minimum
    - [ ] average and
    - [ ] maximum duration, in milliseconds, of the songs in the dataset.
- [ ] Compute
    - [ ] the first and
    - [ ] third quartiles ($Q_1$ and $Q_3$),
    - [ ] as well as the [interquartile range
    (IRQ)](https://en.wikipedia.org/wiki/Interquartile_range) ($Q_3-Q_1$).
- [ ] Compute the set of songs with durations that are not outliers—with
    duration $x$ such that $Q_1-1.5IQR < x < Q_3+1.5IQR$.
- [ ] Using the IQRR methodology, how many songs would be considered
    outliers and removed from analysis?
    - [ ] Generate a new table containing
        - [ ] the minimum
        - [ ] average and
        - [ ] maximum duration of the remaining songs.

### 2. Finding the most popular artists over time

- [ ] find the five most popular artists ranked by the number of
    playlists they appear in.
- [ ] Create a chart that shows the number of
    playlists containing each of these five artists over the years. Consider
    that an artist is present in a playlist after each playlist's last
    modification date.

### 3. Playlists's behavior

What is more common: Playlists where
there are many songs by the same artist or playlists with more diverse
songs?

- [ ] compute the *prevalence* of the most
    frequent artist in each playlist, defined as the fraction of songs by
    the most frequent artist.
- [ ] Then create a [Cumulative Distribution
    Function](https://en.wikipedia.org/wiki/Cumulative_distribution_function)
    (CDF) plot containing the distribution of artist prevalence across all
    playlists.

### What to Submit

- [ ] All code you developed in this project. Organize the code of each
    task so they are easy to identify.
- [ ] A PDF file explaining your solution and findings. Describe the
    approach you took to tackle each task, discuss the results you
    obtained, and report any post-processing (e.g., filtering) you
    applied to the data. Your documentation should include at least in
    addition to the discussion:
    - [ ] Two tables and a paragraph discussing the results for Task 1.
    - [ ] One graph and a paragraph discussing the results for Task 2.
    - [ ] One graph and a paragraph discussing the results for Task 3.
