# MAF Example `simple-platform`

This example is for testing the MAF Platform server and the MAF CLI.

To run this example, you will need to have the MAF Platform server running.
You can start the server by running the following command in the root of the MAF
repository:

```bash
# Configure the environment variables to how you want them. Keep the defaults
# for admin username.
cp .env.example .env

just migrate
just dev-platform
```

Then, in `crates/maf/examples/basic`, run the following command to upload the
example app to the platform server:

```bash
just dev-cli app deploy
```

Finally, run the Vite development server for the example app:

```bash
pnpm dev
```
