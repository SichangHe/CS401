"""Omit until within the with clause when used in `pyspark`.
"""
from contextlib import contextmanager

import matplotlib.pyplot as plt
from matplotlib.axes import Axes
from matplotlib.figure import Figure
from pyspark.sql import SparkSession

plt.rcParams["font.size"] = 24
plt.rcParams["lines.linewidth"] = 4

@contextmanager
def spark_context(spark: SparkSession):
    sc = None
    try:
        yield (sc := spark.sparkContext)
    finally:
        sc.stop() if sc else None


spark = SparkSession.builder.appName("Songs").getOrCreate()
with spark_context(spark) as sc:
    playlist_df = spark.read.json("/datasets/spotify/playlist.json")
    playlist_df.createOrReplaceTempView("playlist")
    track_df = spark.read.json("/datasets/spotify/tracks.json")
    track_df.createOrReplaceTempView("track")

    print("## 1. Statistics about songs duration")

    duration_stats = spark.sql(
        """
select min(duration_ms) as min_duration_ms,
    avg(duration_ms) as avg_duration_ms,
    max(duration_ms) as max_duration_ms
from track"""
    )
    print(
        "Minimum, average, and maximum song duration, in milliseconds, of the songs in the dataset:"
    )
    duration_stats.show()
    min_duration_ms, avg_duration_ms, max_duration_ms = duration_stats.take(1)[0]

    duration_tiles = spark.sql(
        """
with tiles as (
    select percentile(duration_ms, 0.25) as first_quartile,
        percentile(duration_ms, 0.75) as third_quartile
    from track
) select first_quartile, third_quartile,
    (third_quartile - first_quartile) as interquartile_range_duration
from tiles"""
    )
    print("First and third quartiles, and interquartile range (IRQ):")
    duration_tiles.show()
    first_quartile, third_quartile, interquartile_range_duration = duration_tiles.take(
        1
    )[0]

    none_outliers = spark.sql(
        f"""
select * from track
where duration_ms > {first_quartile - 1.5*interquartile_range_duration}
and duration_ms < {third_quartile + 1.5*interquartile_range_duration}"""
    )
    print("Set of songs with durations that are not outliers:")
    none_outliers.show()

    n_outliers = track_df.count() - none_outliers.count()
    print(f"{n_outliers} songs would be considered outliers and removed from analysis.")

    none_outliers.createOrReplaceTempView("none_outlier")
    none_outlier_stats = spark.sql(
        """
select min(duration_ms) as min_duration_ms,
    avg(duration_ms) as avg_duration_ms,
    max(duration_ms) as max_duration_ms
from none_outlier"""
    )
    print(
        "Minimum, average, and maximum song duration, in milliseconds, of the songs of the remaining songs:"
    )
    none_outlier_stats.show()

    print("## 2. Finding the most popular artists over time")

    popular_artists = spark.sql(
        """
select artist_name, artist_uri, count(distinct pid) as count
from playlist join track using (pid)
group by artist_name, artist_uri
order by count(distinct pid) desc
limit 5"""
    )
    print("Five most popular artists ranked by the number of playlists they appear in:")
    popular_artists.show()

    artist_names, artist_uris, counts = list(zip(*popular_artists.take(5)))
    artists_over_time = spark.sql(
        f"""
with playlist_w_track_w_time as (
    select *, year(from_unixtime(modified_at)) as modified_year
    from playlist join track using (pid)
    where artist_uri in ({
        ",".join([f"'{artist_uri}'" for artist_uri in artist_uris])
    })
), timeline as (
    select distinct modified_year as year
    from playlist_w_track_w_time
) select year,
{
    ",".join([
        f"count(distinct case when artist_uri = '{uri}' then pid end) as n_playlist{index}"
        for index, uri in enumerate(artist_uris)
    ])
}
from playlist_w_track_w_time, timeline
where modified_year <= year
group by year
order by year asc"""
    )
    print("Number of playlists containing each of the top five artists over the years:")

    artists_over_time.show()

    artists_over_time_df = artists_over_time.toPandas()
    years = artists_over_time_df["year"]
    fig: Figure
    ax: Axes
    fig, ax = plt.subplots(figsize=(8, 6))  # type: ignore
    ax.set_xlabel("Year")
    ax.set_ylabel("#Playlists Containing Artist")
    ax.set_yscale("log")  # type: ignore
    for index, artist_name in enumerate(artist_names):
        ax.plot(years, artists_over_time_df[f"n_playlist{index}"], label=artist_name)
    ax.legend()
    fig.savefig("artists_over_time.pdf", bbox_inches="tight")
