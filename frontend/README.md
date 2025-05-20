# BRIG-UI Frontend

This is the CSR frontend for the ASV's UI, built in Rust with the Leptos framework.

It handles data visualization and sensor instructions.

The frontend is ran as a NGINX docker container, serving the compiled web assembly.

## Compilation and Deployment

Compile:

`trunk build --release`

Build container:
`docker build -t brig-ui_rpi .`

Save container:
`docker save -o -./brig_container_rpi_RELEASEVERSION.tar brig-ui_rpi`

then upload to the ASV via scp:

`scp ./YOUR_FOLDER_PATH/brig_container_rpi_RELEASEVERSION.tar pi@192.168.2.9:~/web-ui/brig_container_rpi_RELEASEVERSION.tar`

