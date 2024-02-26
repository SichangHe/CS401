"""Serverless function to derive monitoring metrics."""

from datetime import datetime, timedelta


def percentage_outgoing_bytes(metrics: dict, result: dict) -> None:
    """The percentage of outgoing traffic bytes."""
    n_bytes_sent = metrics["net_io_counters_eth0-bytes_sent"]
    n_bytes_received = metrics["net_io_counters_eth0-bytes_recv"]
    assert isinstance(n_bytes_sent, int)
    assert isinstance(n_bytes_received, int)
    result["percentage_outgoing_bytes"] = (
        n_bytes_sent * 100.0 / (n_bytes_sent + n_bytes_received)
    )


def percentage_memory_caching(metrics: dict, result: dict) -> None:
    """The percentage of memory caching content."""
    memory_buffer = metrics["virtual_memory-buffers"]
    memory_cached = metrics["virtual_memory-cached"]
    memory_used = metrics["virtual_memory-total"]
    assert isinstance(memory_cached, int)
    assert isinstance(memory_buffer, int)
    assert isinstance(memory_used, int)
    result["percentage_memory_caching"] = (
        (memory_buffer + memory_cached) * 100.0 / memory_used
    )


def moving_avg_cpu(metrics: dict, context, result: dict) -> None:
    """Compute a moving average utilization of each CPU over the last minute."""
    metrics_timestamp_str = metrics["timestamp"]
    assert isinstance(metrics_timestamp_str, str)
    metrics_timestamp = datetime.fromisoformat(metrics_timestamp_str)
    one_minute_ago = metrics_timestamp - timedelta(minutes=1)

    assert isinstance(context.env, dict)

    for cpu_index in range(8192):
        cpu_key = f"cpu_percent-{cpu_index}"
        if (cpu_percent := metrics.get(cpu_key)) is None:
            break
        assert isinstance(cpu_percent, float)

        cpu_percents_in_last_minute = [
            (percent, time)
            for percent, time in context.env.get(cpu_key, [])
            if time > one_minute_ago
        ]
        cpu_percents_in_last_minute.append((cpu_percent, metrics_timestamp))
        context.env[cpu_key] = cpu_percents_in_last_minute

        result[f"moving_average_{cpu_key}"] = sum(
            percent for percent, _ in cpu_percents_in_last_minute
        ) / len(cpu_percents_in_last_minute)


def handler(metrics: dict, context) -> dict:
    """Entry point for the runtime."""
    result: dict = {}
    percentage_outgoing_bytes(metrics, result)
    percentage_memory_caching(metrics, result)
    moving_avg_cpu(metrics, context, result)
    return result
