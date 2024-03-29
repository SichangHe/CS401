import logging
import os
import sys
from dataclasses import dataclass, field
from datetime import datetime
from importlib import import_module
from importlib.util import module_from_spec, spec_from_file_location
from types import ModuleType

logging.basicConfig(format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger(__name__)
logger.setLevel(os.getenv("PYTHON_LOG", "INFO"))


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


def import_file(file_path: str, module_name: str) -> ModuleType | None:
    """Import a Python file `file_path` as a module named `module_name`."""
    # From <https://docs.python.org/3/library/importlib.html#importing-a-source-file-directly>.
    if (spec := spec_from_file_location(module_name, file_path)) and (
        loader := spec.loader
    ):
        module = module_from_spec(spec)
        sys.modules[module_name] = module
        loader.exec_module(module)
        return module
    else:
        return None


def import_zipped_module(zip_path: str, module_name: str) -> ModuleType | None:
    """Import a Python module `module_name` from the ZIP file at `zip_path`."""
    sys.path.insert(0, zip_path)
    return import_module(module_name)
