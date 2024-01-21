"""Omit until within the with clause when used in `pyspark`.
"""
from contextlib import contextmanager

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.axes import Axes
from matplotlib.figure import Figure
from pyspark.sql import DataFrame, SparkSession

plt.rcParams["font.size"] = 24
plt.rcParams["lines.linewidth"] = 4


def main():
    spark = SparkSession.builder.appName("Songs").getOrCreate()
    with spark_context(spark) as sc:
        sc.setLogLevel("WARN")
        playlist_df = spark.read.json("/datasets/spotify/playlist.json")
        playlist_df.createOrReplaceTempView("playlist")
        track_df = spark.read.json("/datasets/spotify/tracks.json")
        track_df.createOrReplaceTempView("track")
        song_duration_stats(spark, track_df)
        popular_artists(spark)


def song_duration_stats(spark: SparkSession, track_df: DataFrame):
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

    duration_tiles = spark.sql(
        """
with tiles as (
    select percentile(duration_ms, 0.25) as first_quartile,
        percentile(duration_ms, 0.75) as third_quartile
    from track
) select first_quartile, third_quartile,
    (third_quartile - first_quartile) as interquartile_range_duration
from tiles"""
    ).localCheckpoint()
    print("First and third quartiles, and interquartile range (IRQ):")
    duration_tiles.show()
    first_quartile, third_quartile, interquartile_range_duration = duration_tiles.take(
        1
    )[0]

    none_outliers = spark.sql(
        f"""
select * from track
where duration_ms > {first_quartile - 1.5*interquartile_range_duration}
and duration_ms < {third_quartile + 1.5*interquartile_range_duration}
order by duration_ms"""
    ).localCheckpoint()
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


def popular_artists(spark: SparkSession):
    print("## 2. Finding the most popular artists over time")

    popular_artists = spark.sql(
        """
select artist_name, artist_uri, count(distinct pid) as count
from playlist join track using (pid)
group by artist_name, artist_uri
order by count(distinct pid) desc
limit 5"""
    ).localCheckpoint()
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
    ).localCheckpoint()
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
    ax.grid()
    ax.legend()
    fig.savefig("artists_over_time.pdf", bbox_inches="tight")
    print("(Plot saved at `artists_over_time.pdf`.)\n")


def playlist_behavior(spark: SparkSession):
    print("## 3. Playlists’s behavior")

    prevalences = spark.sql(
        """
with artist_track_count as (
    select pid, artist_uri, count(distinct track_uri) as n_track
    from playlist join track using (pid)
    group by pid, artist_uri
) select pid, (max(n_track) / sum(n_track)) as prevalence
from artist_track_count
group by pid
order by (max(n_track) / sum(n_track)) desc"""
    ).localCheckpoint()
    print("The prevalence of the most frequent artist in each playlist:")
    prevalences.show()

    prevalences_df = prevalences.toPandas()
    fig, ax = plt.subplots(figsize=(8, 6))  # type: ignore
    ax.set_xlabel("Prevalences of The Most Frequent\nArtist in Each Playlist")
    ax.set_ylabel("Cumulative Fraction of\nPlaylists")
    ecdf(ax, prevalences_df["prevalence"], label="CDF")
    ax.grid()
    ax.legend()
    fig.savefig("cdf_prevalence_over_playlist.pdf", bbox_inches="tight")
    print("(CDF plot saved at `cdf_prevalence_over_playlist.pdf`.)\n")


@contextmanager
def spark_context(spark: SparkSession):
    sc = None
    try:
        yield (sc := spark.sparkContext)
    finally:
        sc.stop() if sc else None


# Copied from <https://github.com/matplotlib/matplotlib/blob/v3.8.2/lib/matplotlib/axes/_axes.py#L7210-L7310>
# with minimum changes
# This is a function in matplotlib 3.8.
def ecdf(
    self_,
    x,
    weights=None,
    *,
    complementary=False,
    orientation="vertical",
    compress=False,
    **kwargs,
):
    """
    Compute and plot the empirical cumulative distribution function of *x*.

    .. versionadded:: 3.8

    Parameters
    ----------
    x : 1d array-like
        The input data.  Infinite entries are kept (and move the relevant
        end of the ecdf from 0/1), but NaNs and masked values are errors.

    weights : 1d array-like or None, default: None
        The weights of the entries; must have the same shape as *x*.
        Weights corresponding to NaN data points are dropped, and then the
        remaining weights are normalized to sum to 1.  If unset, all
        entries have the same weight.

    complementary : bool, default: False
        Whether to plot a cumulative distribution function, which increases
        from 0 to 1 (the default), or a complementary cumulative
        distribution function, which decreases from 1 to 0.

    orientation : {"vertical", "horizontal"}, default: "vertical"
        Whether the entries are plotted along the x-axis ("vertical", the
        default) or the y-axis ("horizontal").  This parameter takes the
        same values as in `~.Axes.hist`.

    compress : bool, default: False
        Whether multiple entries with the same values are grouped together
        (with a summed weight) before plotting.  This is mainly useful if
        *x* contains many identical data points, to decrease the rendering
        complexity of the plot. If *x* contains no duplicate points, this
        has no effect and just uses some time and memory.

    Other Parameters
    ----------------
    data : indexable object, optional
        DATA_PARAMETER_PLACEHOLDER

    **kwargs
        Keyword arguments control the `.Line2D` properties:

        %(Line2D:kwdoc)s

    Returns
    -------
    `.Line2D`

    Notes
    -----
    The ecdf plot can be thought of as a cumulative histogram with one bin
    per data entry; i.e. it reports on the entire dataset without any
    arbitrary binning.

    If *x* contains NaNs or masked entries, either remove them first from
    the array (if they should not taken into account), or replace them by
    -inf or +inf (if they should be sorted at the beginning or the end of
    the array).
    """
    if "drawstyle" in kwargs or "ds" in kwargs:
        raise TypeError("Cannot pass 'drawstyle' or 'ds' to ecdf()")
    if np.ma.getmask(x).any():
        raise ValueError("ecdf() does not support masked entries")
    x = np.asarray(x)
    if np.isnan(x).any():
        raise ValueError("ecdf() does not support NaNs")
    argsort = np.argsort(x)
    x = x[argsort]
    if weights is None:
        # Ensure that we end at exactly 1, avoiding floating point errors.
        cum_weights = (1 + np.arange(len(x))) / len(x)
    else:
        weights = np.take(weights, argsort)  # Reorder weights like we reordered x.
        cum_weights = np.cumsum(weights / np.sum(weights))
    if compress:
        # Get indices of unique x values.
        compress_idxs = [0, *(x[:-1] != x[1:]).nonzero()[0] + 1]
        x = x[compress_idxs]
        cum_weights = cum_weights[compress_idxs]
    if orientation == "vertical":
        if not complementary:
            (line,) = self_.plot(
                [x[0], *x], [0, *cum_weights], drawstyle="steps-post", **kwargs
            )
        else:
            (line,) = self_.plot(
                [*x, x[-1]], [1, *1 - cum_weights], drawstyle="steps-pre", **kwargs
            )
        line.sticky_edges.y[:] = [0, 1]
    else:  # orientation == "horizontal":
        if not complementary:
            (line,) = self_.plot(
                [0, *cum_weights], [x[0], *x], drawstyle="steps-pre", **kwargs
            )
        else:
            (line,) = self_.plot(
                [1, *1 - cum_weights], [*x, x[-1]], drawstyle="steps-post", **kwargs
            )
        line.sticky_edges.x[:] = [0, 1]
    return line


main() if __name__ == "__main__" else None
