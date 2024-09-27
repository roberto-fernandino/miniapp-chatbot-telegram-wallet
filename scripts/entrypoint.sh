#!/bin/sh

set -e

if echo "$DOCKER_TAGS" | grep -q "dev"; then
  echo "Running in development mode"
  exec npm run dev -- --host
else
  echo "Running in production mode"
  exec npm start
fi
