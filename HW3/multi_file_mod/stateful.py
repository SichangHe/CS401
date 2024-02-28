from datetime import datetime, timedelta


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
