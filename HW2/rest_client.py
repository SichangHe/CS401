#! /usr/bin/env python3
import json
import subprocess
import sys
import time


def print_help():
    print(
        """Usage: rest_client.py [<FLAG>] <Song recommender server IP> <Song recommender server port> <Song 0> [<Song 1> … <Song n>]
<FLAG>:
    -h, --help: Print this help message
    -c, --continuous: Make continuous requests to the server and measure response changes
"""
    )


def try_parse_args():
    continuous = False
    arguments = sys.argv[1:]
    ip = arguments[0]
    if ip in ("-h", "--help"):
        print_help()
        sys.exit(0)
    elif ip in ("-c", "--continuous"):
        continuous = True
        arguments = arguments[1:]
        ip = arguments[0]
    port = arguments[1]
    songs = arguments[2:]
    assert len(songs) > 0, "At least one song must be provided"

    return continuous, ip, port, songs


def parse_args():
    try:
        return try_parse_args()
    except Exception as err:
        print(err)
        print_help()
        sys.exit(1)


def request(post_data, address):
    curl_json_request_args = [
        "curl",
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json",
        "-d",
        post_data,
        "--max-time",
        "0.9",
        address,
    ]

    return subprocess.run(curl_json_request_args, capture_output=True, text=True)


def single_request(post_data, address):
    run_result = request(post_data, address)
    if run_result.returncode != 0:
        print(f"Failed to make request with cURL:\n{run_result.stderr}")
        sys.exit(1)

    result_dict = json.loads(run_result.stdout)
    songs = "\n".join(f"    {song}" for song in result_dict["songs"])
    version = result_dict["version"]
    model_date = result_dict["model_date"]

    print(
        f"""Recommendation server v{version} with model from {model_date}.
Song recommendations:
{songs}"""
    )


def continuous_request(post_data, address):
    print("time,response_time,version,model_date")

    previous_response_time = 0
    previous_version = ""
    previous_model_date = ""
    next_start_time = time.time()

    while True:
        request_start_time = time.time()
        run_result = request(post_data, address)
        response_time = (time.time() - request_start_time) * 1000

        if run_result.returncode == 0:
            result_dict = json.loads(run_result.stdout)
            version = result_dict["version"]
            model_date = result_dict["model_date"]
        else:
            version = ""
            model_date = ""

        if (
            response_time > 1.2 * previous_response_time
            or response_time < 0.8 * previous_response_time
            or version != previous_version
            or model_date != previous_model_date
        ):
            print(
                f"{time.strftime('%H:%M:%S')},{response_time:.3f},{version},{model_date}"
            )

        previous_response_time = response_time
        previous_version = version
        previous_model_date = model_date

        next_start_time += 1
        time_to_sleep = max(0, next_start_time - time.time())
        time.sleep(time_to_sleep)


def main():
    continuous, ip, port, songs = parse_args()
    post_data = json.dumps({"songs": songs})
    address = f"http://{ip}:{port}/api/recommend"
    if continuous:
        continuous_request(post_data, address)
    else:
        single_request(post_data, address)


if __name__ == "__main__":
    try:
        main()
    except InterruptedError:
        sys.exit(1)
