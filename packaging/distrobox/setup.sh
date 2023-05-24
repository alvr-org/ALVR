#!/bin/bash

./setup-process.sh "$@" 2>&1 | tee setup.log || true
cp setup.log /tmp/alvr-setup.log
sed < /tmp/alvr-setup.log $'s/\033[[][^A-Za-z]*m//g' > setup.log