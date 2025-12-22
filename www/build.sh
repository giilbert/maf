#!/bin/bash

# this script builds @usemaf/panel, copies the static files to
# www/public/_panel, and then builds the www next.js website.

# exit immediately if a command exits with a non-zero status.
set -e 

# build @usemaf/panel
echo "[build.sh] panel packages with vite and prerender script"
cd ../packages/panel
pnpm build
echo ""

# cp the build output to www/public/_panel
echo "[build.sh] copy panel dist/prerender and dist/assets to www/public/_panel"
rm -rf ../../www/public/_panel
mkdir -p ../../www/public/_panel
cp -r dist/assets dist/prerender ../../www/public/_panel
echo ""

# build next.js now that panel static files are in place
echo "[build.sh] build www package with pnpm"
cd ../../www
pnpm next:build