#!/bin/bash

# this script builds @usemaf/panel, copies the static files to
# www/public/_panel, and then builds the www next.js website.

# exit immediately if a command exits with a non-zero status.
set -e 

# build @usemaf/panel
echo "[build.sh] panel packages with vite"
cd ../packages/panel
pnpm build
echo ""

# cp the @usemaf/panel/dist to www/public/_panel
echo "[build.sh] copy panel dist to www/public/_panel"
rm -rf ../../www/public/_panel
cp -r dist ../../www/public/_panel
echo ""

# build the www package
echo "[build.sh] build www package with pnpm"
cd ../../www
pnpm next:build