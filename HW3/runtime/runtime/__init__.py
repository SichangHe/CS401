import logging
import os
from dataclasses import dataclass, field
from datetime import datetime

logging.basicConfig(format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.Logger(__name__, level=os.getenv("PYTHON_LOG", "INFO"))


@dataclass(frozen=True)  # NB: Freezing means only `env` is mutable.
class Context:
    host: str
    """Hostname of the server running Redis."""
    port: int
    """Port where the Redis server is listening."""
    input_key: str
    """Input key used to read monitoring data from Redis."""
    output_key: str
    """Output key used to store metrics on Redis."""
    function_getmtime: datetime
    """Timestamp of the last update to the user's module's Python file."""
    last_execution: datetime | None = None
    """Timestamp of last execution of the user's serverless function and result
    storage on Redis."""
    env: dict = field(default_factory=dict)
    """Dictionary to persist data between executions."""
