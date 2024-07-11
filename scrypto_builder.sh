#!/bin/bash

sudo find . -type d -name 'target' -exec rm -rf {} +
DOCKER_DEFAULT_PLATFORM=linux/amd64 docker run -v ./:/src radixdlt/scrypto-builder:v1.2.0
