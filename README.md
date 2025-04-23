Repository for Brigantine's ASV software.

backend --> rust backend for the web interface, handles queries to influx and systemctl commands

frontend --> CSR web interface made with Rust/Leptos, compiled in webassembly and run under a docker container

sensor_modules --> python scripts for the various sensors equipped

    idronaut/idronaut_slim.py --> mainline script for ONLY idronaut data, sends data to influx
    
    gps/gps.py --> script for reading GPS and PingSonar data, sends data to influx
    
    micasense/camera_capture.py --> automatic capture of images via micasense HTTP APIs, sends image path on the SD card to influx
