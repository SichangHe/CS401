from datetime import datetime, timedelta
from typing import Final

ONE_MINUTE: Final[float] = 60.0


def percentage_outgoing_bytes(input: dict, result: dict[str, float]) -> None:
    """The percentage of outgoing traffic bytes."""
    n_bytes_sent = input["net_io_counters_eth0-bytes_sent1"]
    n_bytes_received = input["net_io_counters_eth0-bytes_recv1"]
    assert isinstance(n_bytes_sent, int)
    assert isinstance(n_bytes_received, int)
    result["percentage_outgoing_bytes"] = n_bytes_sent * 100.0 / n_bytes_received


def percentage_memory_caching(input: dict, result: dict[str, float]) -> None:
    """The percentage of memory caching content."""
    memory_buffer = input["virtual_memory-buffers"]
    memory_cached = input["virtual_memory-cached"]
    memory_used = input["virtual_memory-used"]
    assert isinstance(memory_cached, int)
    assert isinstance(memory_buffer, int)
    assert isinstance(memory_used, int)
    result["percentage_memory_caching"] = (
        (memory_buffer + memory_cached) * 100.0 / memory_used
    )


def moving_avg_cpu(input: dict, context, result: dict[str, float]) -> None:
    """Compute a moving average utilization of each CPU over the last minute."""
    now = datetime.now()
    one_minute_ago = now - timedelta(seconds=ONE_MINUTE)

    assert isinstance(context.env, dict)

    for cpu_index in range(8192):
        cpu_key = f"cpu_percent-{cpu_index}"
        cpu_percent: float | None = input.get(cpu_key)
        if cpu_percent is None:
            break

        cpu_percents_in_last_minute: list[tuple[float, datetime]] = [
            (percent, time)
            for percent, time in context.env.get(cpu_key, [])
            if time > one_minute_ago
        ]
        cpu_percents_in_last_minute.append((cpu_percent, now))
        context.env[cpu_key] = cpu_percents_in_last_minute

        result[f"moving_average_{cpu_key}"] = sum(
            percent for percent, _ in cpu_percents_in_last_minute
        ) / len(cpu_percents_in_last_minute)


def handler(input: dict, context) -> dict[str, float]:
    """Entry point for the runtime."""
    result: dict[str, float] = {}
    percentage_outgoing_bytes(input, result)
    percentage_memory_caching(input, result)
    moving_avg_cpu(input, context, result)
    return result
