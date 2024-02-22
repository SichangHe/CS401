import importlib
import json
import os
import traceback
from dataclasses import replace
from datetime import datetime
from time import sleep
from typing import Callable

from redis import Redis

from runtime import Context, logger

REDIS_HOST = os.getenv("REDIS_HOST", "localhost")
REDIS_PORT = int(os.getenv("REDIS_PORT", "6379"))
REDIS_INPUT_KEY = os.getenv("REDIS_INPUT_KEY", "metrics")
REDIS_OUTPUT_KEY: str = os.getenv("REDIS_OUTPUT_KEY")  # type: ignore
assert (
    REDIS_OUTPUT_KEY is not None
), "Environment variable `REDIS_OUTPUT_KEY` must be provided."

FUNCTION_MODULE_NAME = "usermodule"  # TODO: What should this be?
SLEEP_SECONDS = 5.0


def run(
    context: Context, redis: Redis, function: Callable, prev_metrics: dict | None
) -> dict | None:
    """Run the function and returns the new metrics if the metrics changed."""
    assert (metrics_bytes := redis.get(REDIS_INPUT_KEY)) is not None
    new_metrics: dict = json.loads(metrics_bytes)  # type: ignore
    if new_metrics == prev_metrics:
        return None
    else:
        output = function(new_metrics, context)
        redis.set(REDIS_OUTPUT_KEY, json.dumps(output))
        return new_metrics


def main() -> int:
    function_module = importlib.import_module(FUNCTION_MODULE_NAME)
    assert (function_module_file := function_module.__file__) is not None
    function = function_module.handler
    function_mtime = datetime.fromtimestamp(os.path.getmtime(function_module_file))

    context = Context(
        REDIS_HOST, REDIS_PORT, REDIS_INPUT_KEY, REDIS_OUTPUT_KEY, function_mtime
    )
    redis = Redis(host=REDIS_HOST, port=REDIS_PORT)

    n_error_allowed = 3
    metrics: dict | None = None
    while n_error_allowed:
        try:
            maybe_metrics = run(context, redis, function, metrics)
            if maybe_metrics is not None:
                metrics = maybe_metrics
                context = replace(context, last_execution=datetime.now())
            n_error_allowed = 3
        except KeyboardInterrupt:
            return 0
        except Exception as exception:
            n_error_allowed -= 1
            logger.error(
                "Running handler function: %s\n%s",
                exception,
                traceback.format_exc(),
            )

        sleep(SLEEP_SECONDS)  # TODO: This is slightly off.

    logger.error("Too many errors. Exiting.")
    return 1


exit(main()) if __name__ == "__main__" else None
