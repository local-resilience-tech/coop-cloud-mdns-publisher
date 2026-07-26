# Co-op Cloud mDNS Publisher

Publish mDNS records for each [Co-op Cloud](https://coopcloud.tech/) application running on this server so that they can be accessed on the local network.

## Licence

This software is licensed under the ANTI-CAPITALIST SOFTWARE LICENSE (v 1.4). See [LICENSE.txt](./LICENSE.txt) for details.

## Development

### Creating releases

The release commands all require `just`.

If you'll be making releases of this app, you'll need to ensure that you have the relevant tooling by running `just setup`.

You can then try release creation by running `just release-dry patch` (or replace patch with `minor` or `major`).

And create the actual release with `just release patch`.

## Running

## Runtime Dependencies

This CLI app is only designed to be running on linux machines that are running the Avahi service, and have DBus available to communicate with it.

## CLI

Run the binary using:

```bash
ccmdns
```

Or from source code using:

```bash
cargo run
```

This will give help on the availability of CLI commands. In general, the main command to keep running and publish mDNS records is `publish`, eg `cargo run publish`.

## Service

To run this as a systemd service, please install the `.deb` package built from the [latest release on github](https://github.com/local-resilience-tech/coop-cloud-mdns-publisher/releases).

For other linux distributions, it's possible to build your own service setup to run `ccmdns publish` but it would be better if you contact us (via an issue on this repository) so that we can automatically build the package type that you need with each release.