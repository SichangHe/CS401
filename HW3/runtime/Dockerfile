# syntax=docker/dockerfile:1

# Comments are provided throughout this file to help you get started.
# If you need more help, visit the Dockerfile reference guide at
# https://docs.docker.com/engine/reference/builder/

ARG PYTHON_VERSION=3.12.2
FROM python:${PYTHON_VERSION}-slim as base

# Prevents Python from writing pyc files.
# Keeps Python from buffering stdout and stderr.
# Pip, Poetry.
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=on \
    PIP_DEFAULT_TIMEOUT=100 \
    POETRY_VERSION=1.8.0 \
    POETRY_HOME="/opt/poetry" \
    POETRY_NO_INTERACTION=1

ENV PATH="$POETRY_HOME/bin:$PATH" \
    PYTHONPATH="/app:$PYTHONPATH"

# Get cURL.
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the source code into the container.
COPY . .

# Get Poetry and install dependencies.
RUN --mount=type=cache,target=/root/.cache \
    curl -sSL https://install.python-poetry.org | python - && \
    poetry install --no-root --only main

# Run the application.
CMD poetry run python -m runtime.main
