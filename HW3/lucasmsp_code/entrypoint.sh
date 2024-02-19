#!/bin/bash


if test -f "$NEW_REQUERIMENTS"; then
    echo "$NEW_REQUERIMENTS exists."
    pip install --requirement $NEW_REQUERIMENTS  --no-cache-dir
fi

python /opt/app.py
