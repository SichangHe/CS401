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
