#!/bin/sh
# Arrival is native and fail-open; the launcher supplies lazy bootstrap without a hook.
epr device arrive || true
