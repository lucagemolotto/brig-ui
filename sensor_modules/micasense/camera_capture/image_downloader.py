
#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import asyncio
import csv
import logging
from pathlib import Path

# mypy: ignore-errors
import aiohttp

"""
The global logging level. This controls what is logged.
"""
LOGGING_LEVEL = logging.INFO

"""
The minimum image size, in bytes.
"""
MIN_IMAGE_SIZE = 16000

"""
Enable download images for cam1 (red camera).
"""
ENABLE_CAM1 = True
"""
Enable download images for cam2 (blue camera)
"""
ENABLE_CAM2 = True


class Camera:
    name: str
    url: str

    def __init__(self, name: str, url: str):
        super().__init__()
        self.name = name
        self.url = url

        # Setup the debug logger for this camera.
        self.debug_logger = logging.getLogger("camera." + self.name + ".debug")

    async def download_capture(
        self, image_path: str, fallback_addr: str = "http://192.168.10.254"
    ) -> None:
        out_path: Path = Path.cwd() / self.name

        out_path = out_path / image_path[1:]
        out_path.parent.mkdir(parents=True, exist_ok=True)

        if out_path.exists() and out_path.stat().st_size > MIN_IMAGE_SIZE:
            self.debug_logger.info("Image %s is already saved, skipping.", out_path)
            return

        print("aaa", self.url, image_path)
        data: bytes = await self._do_request(self.url + image_path)
        if not data:
            # Try fallback address
            data = await self._do_request(fallback_addr + image_path)
            if data:
                self.debug_logger.debug(
                    "Use fallback address %s to request the image %s",
                    fallback_addr,
                    image_path,
                )

        if not data:
            self.debug_logger.error("Failed to get image '%s'", self.url + image_path)
            return

        with open(out_path, "wb") as file:
            file.write(data)

        self.debug_logger.info(
            "Successfully saved file '%s' to '%s' from cam %s",
            image_path,
            out_path,
            self.name,
        )

    async def _do_request(self, url: str) -> bytes:
        timeout = aiohttp.ClientTimeout(total=10)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            try:
                async with session.get(url) as resp:
                    if not resp.ok:
                        self.debug_logger.error(
                            "Request %s fails with error %d", resp.url, resp.status
                        )
                        return b""

                    data = await resp.read()

                    return data

            except OSError:
                self.debug_logger.error(
                    "Failed to connect to %s. Check if camera is active and connected.",
                    url,
                )
                return b""
            except asyncio.TimeoutError:
                self.debug_logger.warning(
                    "Request %s was not completed in time."
                    + "Check if camera is active and connected.",
                    url,
                )
                return b""
            except Exception as ex:
                self.debug_logger.error("Request %s fails", url, exc_info=ex)
                return b""


def _setup_logging() -> None:
    logging.basicConfig(level=LOGGING_LEVEL)

    debug_formatter = logging.Formatter(
        "%(asctime)s %(name)s:%(levelname)s: %(message)s", datefmt="%Y-%m-%d %H:%M:%S"
    )
    debug_file_handler = logging.FileHandler("image_downloader.log")
    debug_file_handler.setFormatter(debug_formatter)
    logging.getLogger().addHandler(debug_file_handler)

    # This formatter seems to be added already, ignore it.
    # debug_print_handler = logging.StreamHandler(sys.stdout)
    # debug_print_handler.setFormatter(debug_formatter)
    # logging.getLogger().addHandler(debug_print_handler)


async def main() -> None:
    # Initialize the loggers
    _setup_logging()
    logger = logging.getLogger("main")

    # Initialize the cameras

    if ENABLE_CAM1:
        cam1 = Camera("cam1", "http://192.168.1.83:80")
    if ENABLE_CAM2:
        cam2 = Camera("cam2", "http://192.168.2.83:80")

    # Set the event file.
    event_file = Path.cwd() / "events.csv"
    if not event_file.exists():
        logger.error("Missing event file 'events.csv'. Try run the camera script.")
        return

    with open(event_file, "r") as events:
        for event in csv.reader(events):
            # Find which camera produces the image set.
            cam = None
            if event[0] == "cam1" and ENABLE_CAM1:
                cam = cam1
            elif event[0] == "cam2" and ENABLE_CAM2:
                cam = cam2
            else:
                logger.error("Invalid camera name %s", event[0])
                continue

            # Iterate over the 5 images.
            # NOTE: Assume that there are exactly 5 images.
            download_feature = asyncio.gather(
                *(cam.download_capture(image) for image in event[-5:])
            )
            await asyncio.wait([download_feature])


if __name__ == "__main__":
    try:
        # Run the main function.
        asyncio.run(main())
    except RuntimeError:
        # Loop already running, just push a Future.
        asyncio.ensure_future(main())
