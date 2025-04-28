#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
Tell the MicaSense cameras to do captures, in sync.

This script produces 2 log files:
    - A debug log.
    - An event log, as CSV.

The event log columns are:
    - camera_name
    - timestamp
    - gps_latitude
    - gps_longitude
    - image_path_1
    - image_path_2
    - ...
    - image_path N

CSV header line:
camera name,timestamp,gps_latitude,gps_longitude,image1,image2,image3,image4,image5,image6,image7,image8,image9,image10

This script uses the Python async infrastructure (async/await and asyncio).
NO threads are used.
"""

import asyncio
import datetime
import json
import logging
import socket
import time
from dataclasses import dataclass, field
from typing import Any, Collection, Optional

# mypy: ignore-errors
import aiohttp
import influxdb_client
from influxdb_client.client.write_api import SYNCHRONOUS

GPSPosition = tuple[str, str]

"""
The address to the GPS TCP socket.
"""
GPS_SOCKET_ADDRESS = ("localhost", 12345)

"""
If True, the system time, logged in the event log, is saved as Unix timestamp.
If False, the system time, logged in the event log, is saved in ISO format.
"""
TIME_USE_TIMESTAMP = False

"""
The global logging level. This controls what is logged.
"""
LOGGING_LEVEL = logging.INFO


"""
The time between a camera capture and the next one.
In normal conditions, this interval is always respected,
even if the capture takes less time.
"""
CAPTURE_INTERVAL = 5


@dataclass
class CaptureResult:
    """Data collected after a camera capture."""

    source_camera: str
    image_paths: Collection[str] = field(default_factory=list)
    gps_position: GPSPosition = ("0", "0")


class Camera:
    """Request data to the camera via HTTP."""

    name: str
    url: str
    expected_images: int = 1

    debug_logger: logging.Logger

    def __init__(self, name: str, url: str, expected_images: int = 1):
        super().__init__()
        self.name = name
        self.url = url
        self.expected_images = expected_images

        # Setup the debug logger for this camera.
        self.debug_logger = logging.getLogger("camera." + self.name + ".debug")

    async def do_capture(self) -> Optional[CaptureResult]:
        """Ask the camera to do a capture."""
        capture_url = self.url + "/capture?block=true"
        timeout = aiohttp.ClientTimeout(4)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            try:
                capture = CaptureResult(self.name)
                capture.image_paths = await self._get_image_urls(session)

                if capture and capture.image_paths:
                    self.debug_logger.debug("Request %s done successfully", capture_url)

                return capture

            except asyncio.TimeoutError:
                self.debug_logger.warning(
                    "Request %s was not completed in time."
                    + "Check if camera is active and connected.",
                    capture_url,
                )
            except OSError:
                self.debug_logger.error(
                    "Failed to connect to %s. Check if camera is active and connected.",
                    self.url,
                )
            except Exception as ex:
                self.debug_logger.error("Request %s fails", capture_url, exc_info=ex)

        # Return None if something goes bad.
        return None

    async def _get_image_urls(
        self, session: aiohttp.ClientSession
    ) -> Optional[Collection[str]]:
        async with session.get(self.url + "/capture?block=true") as resp:
            if not resp.ok:
                self.debug_logger.error(
                    "Request %s fails with error %d", resp.url, resp.status
                )
                return None

            response_json: dict
            try:
                response_json = await resp.json()
            except Exception as ex:
                self.debug_logger.error(
                    "Response from %s is not a JSON", resp.url, exc_info=ex
                )
                return None

            if response_json.get("status", "error") != "complete":
                self.debug_logger.error(
                    "Camera fails to capture the image. Status: %s",
                    response_json.get("status"),
                )
                return None

            raw_paths = response_json.get("raw_storage_path", {})
            raw_path_len = len(raw_paths)
            if raw_path_len == 0:
                self.debug_logger.error("Camera returned no image paths")
                return None

            if raw_path_len != self.expected_images:
                self.debug_logger.warning(
                    "Camera did not return the expected number of images (%d),"
                    + "but it returns %d images",
                    self.expected_images,
                    raw_path_len,
                )

            # All check passed, return the image paths.
            return tuple(raw_paths.values())


class GPSSocketCommunication:
    """
    Communicate with the GPS service.

    Protocol usage:
        - Connect with TCP
        - Send message "GPS"
        - Receive a JSON with the following structure:
          `{"latitude": number, "longitude": number}`
    """

    gps_address: tuple[Any, Any]
    _last_gps_pos: GPSPosition

    debug_logger: logging.Logger

    def __init__(self, gps_address: tuple[Any, Any]):
        super().__init__()
        self.gps_address = gps_address
        self._last_gps_pos = ("0", "0")

        # Initialize the debug logger.
        self.debug_logger = logging.getLogger("gps_communication")

    async def get_gps_position(self) -> Optional[GPSPosition]:
        """Get the GPS position from the GPS socket."""
        self.debug_logger.debug("Start GPS position comunication")

        loop = asyncio.get_running_loop()

        transport: asyncio.BaseTransport
        protocol: asyncio.Protocol
        socket_type = "TCP"
        try:
            # Create a TCP socket.
            # NOTE: The base protocol can be changed by changing this statement.

            transport, protocol = await loop.create_connection(
                asyncio.Protocol, host=self.gps_address[0], port=self.gps_address[1]
            )
        except (ConnectionRefusedError, OSError):
            self.debug_logger.error(
                "Failed to connect to %s:%s, using %s. Check if GPS service is active.",
                self.gps_address[0],
                self.gps_address[1],
                socket_type,
            )
            return None
        except Exception as ex:
            self.debug_logger.error(
                "Failed to create the %s:%s transport, using %s",
                self.gps_address[0],
                self.gps_address[1],
                socket_type,
                exc_info=ex,
            )
            return None

        # Get the socket object to do the required operations.
        sock: socket.socket = transport.get_extra_info("socket")

        # Send the GPS position request (message GPS)
        try:
            await loop.sock_sendall(sock, b"GPS")
        except OSError:
            self.debug_logger.error("Connection closed, can't send GPS request.")
            transport.close()
            return None
        except Exception as ex:
            self.debug_logger.error("Failed to send GPS request", exc_info=ex)
            transport.close()
            return None

        try:
            raw_data: bytes = await loop.sock_recv(sock, 1 << 16)  # 16k
        except OSError:
            self.debug_logger.error("Connection closed, can't get GPS data.")
            transport.close()
            return None
        except Exception as ex:
            self.debug_logger.error("Failed to get GPS data", exc_info=ex)
            transport.close()
            return None

        transport.close()
        try:
            gps_data: dict = json.loads(raw_data)
        except Exception as ex:
            self.debug_logger.error("Failed to parse message", exc_info=ex)
            return self._last_gps_pos

        self.debug_logger.debug("Connection to GPS socket done, response: %s", gps_data)

        # Assume that the data fields are correct and don't check for errors.
        # In the worst case, (0, 0) is the GPS position
        new_gps_pos: GPSPosition = (gps_data.get("lat", "0"), gps_data.get("lon", "0"))

        # If the given position is ("0", "0"), return the last one.
        if new_gps_pos != ("0", "0"):
            self.debug_logger.debug("GPS position returned successfully")
            self._last_gps_pos = new_gps_pos
        else:
            self.debug_logger.warning(
                "GPS position is missing or malformed, return the last one."
            )
        return self._last_gps_pos

    def get_last_position(self) -> GPSPosition:
        """
        Get the last known GPS position.
        """
        return self._last_gps_pos


class CameraArray:
    """Control the cameras and store the resulting events."""

    cameras: list[Camera]
    gps_comm: Optional[GPSSocketCommunication]

    event_logger: logging.Logger
    debug_logger: logging.Logger

    _last_images: Collection[Optional[CaptureResult]]

    influx_org: str
    write_api: influxdb_client.client.write_api.WriteApi

    def __init__(
        self, *cameras: Camera, gps_comm: Optional[GPSSocketCommunication] = None
    ):
        super().__init__()
        self.cameras = list(cameras)
        self.gps_comm = gps_comm
        self._last_images = []

        self._setup_logging()

        self.influx_org = "SailingLab"
        client = influxdb_client.InfluxDBClient(
            url="http://localhost:8086",
            token="ijL6ry3VP0Hm5nAvP-wvHouC1l3ysIWty-VWCPgF7Bz-aKt-4Oi9zFMV_t8UkVnQSVwdxlRpdKjbAuPxx9umsA==",
            org=self.influx_org,
        )
        self.write_api = client.write_api(write_options=SYNCHRONOUS)

    def _setup_logging(self) -> None:
        # Initialize the event logging file.
        self.event_logger = logging.getLogger("camera_array")

        event_handler = logging.FileHandler("events.csv")

        self.event_logger.addHandler(event_handler)
        self.event_logger.propagate = False

        # Initialize the debug logger
        self.debug_logger = logging.getLogger("camera_array_debug")

    async def do_capture(self) -> Collection[Optional[CaptureResult]]:
        """Do the captures for all the cameras."""

        # Doing the captures asynchronously allow them to be done in parallel
        # (well, almost).
        self.debug_logger.debug("Start capture tasks")
        capture_future = asyncio.gather(
            *(camera.do_capture() for camera in self.cameras)
        )
        gps_position = (0, 0)
        if self.gps_comm:
            gps_future = asyncio.ensure_future(self.gps_comm.get_gps_position())
            
            # Wait for capture tasks.
            completed, _ = await asyncio.wait([capture_future, gps_future])
            
            # Get the GPS position
            gps_position: Optional[GPSPosition] = gps_future.result()
            if gps_position is None:
                gps_position = self.gps_comm.get_last_position()
        else:
            completed, _ = await asyncio.wait([capture_future])

        captures: Collection[Optional[CaptureResult]] = []
        for capture in capture_future.result():
            capture: Optional[CaptureResult]

            if not capture:
                self.debug_logger.debug("No capture, continue.")
                continue

            captures.append(capture)

            # Get the system time using the selected format.
            current_time: str = "0"
            if TIME_USE_TIMESTAMP:
                current_time = str(int(time.time()))
            else:
                current_time = datetime.datetime.now().strftime("%y-%m-%d %H-%M-%S")

            if LOGGING_LEVEL >= logging.INFO:
                # Print data to show that it's working.
                print(
                    capture.source_camera,
                    current_time,
                    # *gps_position,
                    *capture.image_paths,
                    sep=", ",
                )

            self.event_logger.info(
                # "%s,%s,%s,%s,%s",
                "%s,%s",
                capture.source_camera,
                # current_time,
                # gps_position[0],
                # gps_position[1],
                ",".join(capture.image_paths),
            )
            self.send_to_influx(capture.source_camera, capture.image_paths)

        self._last_images = captures
        return captures

    def send_to_influx(self, source_camera: str, image_paths: list[str]) -> None:
        p = influxdb_client.Point("micasense_data").field("capture", image_paths[0]).tag("camera", source_camera)
        self.write_api.write(bucket="asv_data", org=self.influx_org, record=p)


def _setup_logging() -> None:
    logging.basicConfig(level=LOGGING_LEVEL)

    debug_formatter = logging.Formatter(
        "%(asctime)s %(name)s:%(levelname)s: %(message)s", datefmt="%Y-%m-%d %H:%M:%S"
    )
    debug_file_handler = logging.FileHandler("camera_capture.log")
    debug_file_handler.setFormatter(debug_formatter)
    logging.getLogger().addHandler(debug_file_handler)

    # This formatter seems to be added already, ignore it.
    # debug_print_handler = logging.StreamHandler(sys.stdout)
    # debug_print_handler.setFormatter(debug_formatter)
    # logging.getLogger().addHandler(debug_print_handler)


async def main(capture_interval: int = 5, request_timeout: int = 5):
    # Initialize the loggers
    _setup_logging()
    logger = logging.getLogger("main")

    # Initialize the cameras and the array
    cam1 = Camera("cam1", "http://192.168.1.83:80", request_timeout)
    cam2 = Camera("cam2", "http://192.168.3.83:80", request_timeout)

    # gps_comm = GPSSocketCommunication(GPS_SOCKET_ADDRESS)

    array = CameraArray(cam1, cam2, gps_comm=None)

    # Start the capture loop
    loop = asyncio.get_event_loop()
    running = True
    while running and loop.is_running():
        try:
            logger.debug("Begin capture cycle")
            start_time = time.monotonic()

            # All the expensive processing is here.
            await array.do_capture()

            end_time = time.monotonic()
            time_to_complete = end_time - start_time
            logger.debug("Time to complete: %f", time_to_complete)

            # This sleep ensures that the processing time plus the waiting times
            # is equals to capture_interval (save for precision issues).
            if time_to_complete < capture_interval:
                await asyncio.sleep(capture_interval - time_to_complete)

            # Let the absolute error be 5ms.
            elif time_to_complete - 0.005 > capture_interval:
                logger.warning(
                    "Processing time exceed the capture interval. Expect longer times"
                )

        except Exception as ex:
            logger.error("Capture cycle fails", exc_info=ex)
            # DEBUG: Abort on error for debugging.
            # running = False

        except BaseException:
            logger.debug("Request to stop")
            running = False


if __name__ == "__main__":
    try:
        # Run the main function.
        asyncio.run(
            main(capture_interval=CAPTURE_INTERVAL, request_timeout=CAPTURE_INTERVAL)
        )
    except RuntimeError:
        # Loop already running, just push a Future.
        asyncio.ensure_future(
            main(capture_interval=CAPTURE_INTERVAL, request_timeout=CAPTURE_INTERVAL)
        )
