# Co-op Cloud mDNS Publisher

Publish mDNS records for each [Co-op Cloud](https://coopcloud.tech/) application running on this server so that they can be accessed on the local network.

## Licence

This software is licensed under the ANTI-CAPITALIST SOFTWARE LICENSE (v 1.4). See [LICENSE.txt](./LICENSE.txt) for details.

## Runtime Dependencies

This CLI app is only designed to be running on linux machines that are running the Avahi service, and have DBus available to communicate with it.

## Development

### Creating releases

The release commands all require `just`.

If you'll be making releases of this app, you'll need to ensure that you have the relevant tooling by running `just setup`.

You can then try release creation by running `just release-dry patch` (or replace patch with `minor` or `major`).

And create the actual release with `just release patch`.