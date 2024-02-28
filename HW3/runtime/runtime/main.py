import json
import os
import traceback
from dataclasses import replace
from datetime import datetime
from time import sleep
from typing import Callable, Final

from redis import Redis

from runtime import Context, import_file, import_zipped_module, logger

REDIS_HOST: Final = os.getenv("REDIS_HOST", "localhost")
REDIS_PORT: Final = int(os.getenv("REDIS_PORT", "6379"))
REDIS_INPUT_KEY: Final = os.getenv("REDIS_INPUT_KEY", "metrics")
REDIS_OUTPUT_KEY: Final = os.getenv("REDIS_OUTPUT_KEY", "sh623-proj3-output")

FUNCTION_MODULE_NAME: Final = "function"
FUNCTION_PATH: Final = os.getenv("FUNCTION_PATH", "/opt/usermodule.py")

FUNCTION_ZIP_PATH: Final = os.getenv("FUNCTION_ZIP_PATH", "/opt/function_module.zip")
ZIPPED_MODULE_NAME: Final = os.getenv("ZIPPED_MODULE_NAME")

HANDLER_FUNCTION_NAME: Final = os.getenv("HANDLER_FUNCTION_NAME", "handler")

POLL_INTERVAL_SECONDS: Final = float(os.getenv("POLL_INTERVAL_SECONDS", "5.0"))
MAX_N_ERROR_ALLOWED: Final = 3


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


def get_function_and_mtime() -> tuple[Callable, datetime] | None:
    if ZIPPED_MODULE_NAME:
        logger.info("Importing zipped module named `%s`.", ZIPPED_MODULE_NAME)
        function_module = import_zipped_module(FUNCTION_ZIP_PATH, ZIPPED_MODULE_NAME)
        function_path = FUNCTION_ZIP_PATH
    else:
        logger.warning(
            "No zipped module name provided. Importing from file `%s`.", FUNCTION_PATH
        )
        function_module = import_file(FUNCTION_PATH, FUNCTION_MODULE_NAME)
        function_path = FUNCTION_PATH
    if function_module is None:
        logger.error(
            "Please provide the function module file. Not found at %s.",
            FUNCTION_PATH,
        )
        return None
    function_mtime = datetime.fromtimestamp(os.path.getmtime(function_path))
    return getattr(function_module, HANDLER_FUNCTION_NAME), function_mtime


def main() -> int:
    if (function_and_mtime := get_function_and_mtime()) is None:
        logger.error("No function module file found.")
        return 1
    function, function_mtime = function_and_mtime

    context = Context(
        REDIS_HOST, REDIS_PORT, REDIS_INPUT_KEY, REDIS_OUTPUT_KEY, function_mtime
    )
    redis = Redis(host=REDIS_HOST, port=REDIS_PORT)

    n_error_allowed = MAX_N_ERROR_ALLOWED
    metrics: dict | None = None
    while n_error_allowed:
        try:
            logger.info("`run`: context=%s, metrics=%s", context, metrics)
            maybe_metrics = run(context, redis, function, metrics)
            if maybe_metrics is not None:
                metrics = maybe_metrics
                logger.info(
                    "Post-run: context.env=%s, metrics=%s", context.env, metrics
                )
                context = replace(context, last_execution=datetime.now())
            n_error_allowed = MAX_N_ERROR_ALLOWED
        except KeyboardInterrupt:
            return 0
        except Exception as exception:
            n_error_allowed -= 1
            logger.error(
                "Running handler function: %s\n%s",
                exception,
                traceback.format_exc(),
            )

        sleep(POLL_INTERVAL_SECONDS)  # TODO: This is slightly off.

    logger.error("Too many errors. Exiting.")
    return 1


exit(main()) if __name__ == "__main__" else None
