# Project 1: Big Data Programming Paradigms

## Dataset Description

The dataset used is in HDFS at `hdfs://localhost:9000/datasets/spotify/`.
`playlist.json` (without "s") containing playlist metadata while `tracks.json`
containing information about songs present in the playlists.

## Tasks

### 1. Statistics about songs duration

Interquartile Range Rule
([IQRR](https://www.thoughtco.com/what-is-the-interquartile-range-rule-3126244)),
a technique that removes points outside an interval defined by the 1st
and 3rd quartiles.

All parts in this tasks are solved using simple SQL queries.
The only exception to using pure SQL is calculating the bounds of
the IQRR interval.
Since we already know $Q1$ and $Q3$ at that point,
I just retrieve them from the previous data frame and use
the calculated interval to construct the SQL query.

(1) Minimum, average, and maximum song duration, in milliseconds, of the songs in the dataset:

|min_duration_ms|   avg_duration_ms|max_duration_ms|
|---------------|------------------|---------------|
|              0|234408.54976216817|       10435467|

(2) First and third quartiles, and interquartile range (IRQ):

|first_quartile|third_quartile|interquartile_range_duration|
|--------------|--------------|----------------------------|
|      198333.0|      258834.0|                     60501.0|

(3) Using the IQRR methodology,
559989 songs would be considered outliers and removed from analysis.

(4) Minimum, average, and maximum song duration, in milliseconds, of the songs of the remaining songs:

|min_duration_ms|   avg_duration_ms|max_duration_ms|
|---------------|------------------|---------------|
|         107582|226899.35353939075|         349583|

The 559989 songs are only 5.20% among all the songs,
but removing them reduced the average song duration by 3.20%.
This shows that some extremely long songs caused significant bias in the average
towards the larger direction.

The minimum and maximum song duration after removing the outliers are
just at the boundaries of the IQRR interval $(107581.5, 349585.5)$.
This suggests that duration values in the dataset may vary a lot such that
they occupy most of the IQRR interval.

### 2. Finding the most popular artists over time

Five most popular artists ranked by the number of playlists they appear in:

|   artist_name|          artist_uri|count|
|--------------|--------------------|-----|
|         Drake|spotify:artist:3T...|32258|
|       Rihanna|spotify:artist:5p...|23963|
|    Kanye West|spotify:artist:5K...|22464|
|    The Weeknd|spotify:artist:1X...|20046|
|Kendrick Lamar|spotify:artist:2Y...|19159|

Number of playlists containing each of the top five artists over the years:

![Number of Playlists Containing The Five Most Popular Artists Over The Years.](artists_over_time.pdf)

While finding the most popular artists is another straightforward SQL query,
getting their appearance over the years indicate that the resulting data frame
needs to have a column for each popular artist.
To achieve this, I manipulated `artist_uri` strings to generate an SQL query
agnostic of the list of popular artists chosen.
Additionally, I only include the years where any of these artist appeared in
at least one playlist.
These results are then converted to `pandas.DataFrame` to construct the plot in
`matplotlib`.

All five popular artists grew exponentially in terms of the number of
playlists they appear in over the years,
in similar fashions.
This indicates that artists' popularity grow in a rather constant rate
each year compared to the last year.

### 3. Playlists's behavior

![CDF of Artists Prevalences Across All Playlists.](cdf_prevalence_over_playlist.pdf)

This yet another SQL query that first calculates the track counts for
each playlist and artist,
then use this information to find the track counts of the prevalent artists
and all the artists combined.
This is built on the assumption that each track has exactly one artist,
which seems to be the case in this schema.
Also, duplicated `track_uri`s are ignored; surprisingly, they exist.

Playlists with diverse songs from various artists are more common.
As the CDF shows, almost 80% of playlists have a prevalence of 20% or less,
and playlists with over 80% prevalence are less than 3%.
This indicates that playlists usually have songs from various artists,
without having any of the artists dominating the playlist.

### Notes

Attached `main.py` is the main script that runs all the tasks;
it invoked by `run.sh`.
An sample output `.txt` file is also attached for reference purposes.
