#! /usr/bin/env python3
import json
import subprocess
import sys


def print_help():
    print(
        "Usage: rest_client.py <Song recommender server IP> <Song recommender server port> <Song 0> [<Song 1> … <Song n>]"
    )


def try_parse_args() -> tuple[str, str, list[str]]:
    ip = sys.argv[1]
    if ip in ("-h", "--help"):
        print_help()
        sys.exit(0)
    port = sys.argv[2]
    songs = sys.argv[3:]
    assert len(songs) > 0, "At least one song must be provided"

    return ip, port, songs


def parse_args() -> tuple[str, str, list[str]]:
    try:
        return try_parse_args()
    except Exception as err:
        print(err)
        print_help()
        sys.exit(1)


def main():
    ip, port, songs = parse_args()
    post_data = json.dumps({"songs": songs})
    curl_json_request_args = [
        "curl",
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json",
        "-d",
        post_data,
        f"http://{ip}:{port}/api/recommend",
    ]

    run_result = subprocess.run(curl_json_request_args, capture_output=True, text=True)
    if run_result.returncode != 0:
        print(f"Failed to make request with cURL:\n{run_result.stderr}")
        sys.exit(1)

    result_dict = json.loads(run_result.stdout)
    songs = "\n".join(f"\t{song}" for song in result_dict["songs"])
    version = result_dict["version"]
    model_date = result_dict["model_date"]

    print(
        f"""Recommendation server v{version} with model from {model_date}.
Song recommendations:
{songs}"""
    )


if __name__ == "__main__":
    try:
        main()
    except InterruptedError:
        sys.exit(1)
