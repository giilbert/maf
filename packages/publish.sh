# GO INCREMENT VERSIONS OF PACKAGES BEFORE RUNNING CUZ WE ARE PUBLISHING ALPHA VERSIONS
cd client
pnpm publish --access public
cd ../react
pnpm publish --access public
cd ../