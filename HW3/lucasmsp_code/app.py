# -*- coding: utf-8 -*-

import json
import os
import time
import traceback
from logging import getLogger

import function as lf
import redis

REDIS_HOST = os.getenv("REDIS_HOST", "localhost")
REDIS_PORT = int(os.getenv("REDIS_PORT", 6379))
REDIS_INPUT_KEY = os.getenv("REDIS_INPUT_KEY", "metrics")
REDIS_OUTPUT_KEY = os.getenv("REDIS_OUTPUT_KEY", "sh623-proj3-output")

INTERVAL_TIME = int(os.getenv("INTERVAL", 5))

logger = getLogger(__name__)


def log(*args):
    logger.warning(*args)


if not REDIS_INPUT_KEY:
    log("ENV `REDIS_INPUT_KEY` must be informed.")
    exit(1)


if not REDIS_OUTPUT_KEY:
    log("ENV `REDIS_OUTPUT_KEY` not informed. Any output will not be sent to Redis.")
    log(os.environ)

r_server = redis.Redis(
    host=REDIS_HOST, port=REDIS_PORT, charset="utf-8", decode_responses=True
)

log("Environment is loaded. Starting Serverless function execution...")

from .context import Context

context = Context(
    host=REDIS_HOST,
    port=REDIS_PORT,
    input_key=REDIS_INPUT_KEY,
    output_key=REDIS_OUTPUT_KEY,
)

while True:
    data = None
    output = None
    try:
        # data = r_server.xrevrange(REDIS_INPUT_KEY, count=1)[0]
        data = r_server.get(REDIS_INPUT_KEY)
    except:
        log("Data not available yet!")
        log(traceback.format_exc())

    # removing from tuple
    if data:

        try:
            # data = json.loads(data[1]['msg'])
            data = json.loads(data)  # type: ignore
            log("\ndata=%s,\ncontext.env=%s", data, context.env)
            output = lf.handler(data, context)
            log("context.env=%s,\noutput=%s", context.env, output)

        except:
            log(
                "Error in Serverless function. Please check your `handler` method in usermodule.py"
            )
            log(traceback.format_exc())

        try:
            if output and REDIS_OUTPUT_KEY:
                # after stream-node-max-bytes (4kb) or stream-node-max-entries (100),
                # this topic will be trimmed to 1 entry
                # r_server.xadd(name=REDIS_OUTPUT_KEY,
                #              fields={"msg": json.dumps(output)},
                #              maxlen=1)
                r_server.set(REDIS_OUTPUT_KEY, json.dumps(output))
            context.confirm_execution()
        except:
            log("Error while trying to save result")
            log(traceback.format_exc())

    time.sleep(INTERVAL_TIME)
