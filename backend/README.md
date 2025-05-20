# BRIG-UI Backend

This is the backend for the ASV's UI, built in Rust.

It handles queries to InfluxDB and to the Micasense cameras.

The backed is ran as systemctl service, called `brig_backend`.

## Compilation and Deployment

Compilation is done with [cross-rs](https://github.com/cross-rs/cross), to generate an executable for Linux AARCH64.
Compile:

`cross build --target aarch64-unknown-linux-musl --release`

stop the service on the ASV:

`sudo systemctl stop brig_backend`

then upload to the ASV via scp:

`scp ./YOUR_FOLDER_PATH/target/aarch64-unknown-linux-musl/release/backend pi@192.168.2.9:~/web-ui/backend/backend`

and restart the service:

`sudo systemctl start brig_backend`

