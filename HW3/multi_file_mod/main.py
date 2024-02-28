from multi_file_mod.stateful import moving_avg_cpu
from multi_file_mod.stateless import (
    percentage_memory_caching,
    percentage_outgoing_bytes,
)


def handler(metrics: dict, context) -> dict:
    """Entry point for the runtime."""
    result: dict = {}
    percentage_outgoing_bytes(metrics, result)
    percentage_memory_caching(metrics, result)
    moving_avg_cpu(metrics, context, result)
    return result
