#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import asyncio
import logging
from pathlib import Path
import json
import shutil
import argparse
import re
from typing import Optional, Union, Literal

# mypy: ignore-errors
import aiohttp

"""
The global logging level. This controls what is logged.
"""
LOGGING_LEVEL = logging.DEBUG

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

CAMERA_ACTION = Union[Literal["download"], Literal["delete"]]

class Camera:
    name: str
    url: str
    name_filter: Optional[re.Pattern] = None
    action: CAMERA_ACTION

    def __init__(self, name: str, url: str, name_filter: str = "", action: CAMERA_ACTION = "download"):
        super().__init__()
        self.name = name
        self.url = url
        if name_filter:
            self.name_filter = re.compile(name_filter)
        self.action = action

        # Setup the debug logger for this camera.
        self.debug_logger = logging.getLogger("camera." + self.name + ".debug")

    async def download_capture(
        self, image_path: str, fallback_addr: str = "http://192.168.10.254"
    ) -> None:
        out_path: Path = Path.cwd() / self.name

        out_path = out_path / image_path[1:]
        out_path.parent.mkdir(parents=True, exist_ok=True)

        # Check if the image already exists and it's valid (big enough).
        if out_path.exists() and out_path.stat().st_size > MIN_IMAGE_SIZE:
            self.debug_logger.info("Image %s is already saved, skipping.", out_path)
            return

        # Don't download images if filter doesn't match.
        if self.name_filter and not self.name_filter.search(str(out_path)):
            self.debug_logger.debug("Image %s filtered out with filter '%s'", str(out_path), self.name_filter.pattern)
            return

        data: bytes = await self._do_request(self.url + "/files/" + image_path)
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

        if out_path.is_dir():
            shutil.rmtree(out_path)

        with open(out_path, "wb") as file:
            file.write(data)

        self.debug_logger.info(
            "Successfully saved file '%s' to '%s' from cam %s",
            image_path,
            out_path,
            self.name,
        )

    async def delete_capture(
        self, image_path: str, fallback_addr: str = "http://192.168.10.254"
    ) -> None:

        # Don't delete images if filter doesn't match.
        if self.name_filter and not self.name_filter.search(str(image_path)):
            self.debug_logger.debug("Image %s filtered out with filter '%s'", str(image_path), self.name_filter.pattern)
            return

        data: bytes = await self._do_request(self.url + "/deletefile/" + image_path)

        if not data:
            self.debug_logger.error("Failed to delete image '%s'", self.url + image_path)
            return

        self.debug_logger.info(
            "Successfully delete file '%s' from cam %s",
            image_path,
            self.name,
        )

    async def get_file_tree(self, route: str = "/files") -> dict:
        r = await self._do_request(self.url + route)
        if not r:
            return {}

        response_json = json.loads(r)
        entries = {}
        self.debug_logger.debug("Enter in folder %s", route)
        for directory in response_json["directories"]:
            if directory == "System Volume Information" or not directory:
                # A really useful folder to return, uh.
                continue
            entries[directory] = await self.get_file_tree(route + "/" + directory)

        for file_entry in response_json["files"]:
            if not file_entry or ".dat" in file_entry["name"]:
                continue
            entries[file_entry["name"]] = file_entry

        return entries

    async def walk_tree(self, tree: dict, parent: str = "") -> None:
        images = []
        for dir, content in tree.items():
            if isinstance(content, dict):
                await self.walk_tree(content, parent + "/" + dir)
            elif isinstance(content, str):
                images.append(parent)
        
        clb = self.download_capture if self.action == "download" else self.delete_capture
        task = asyncio.gather(*(clb(image) for image in images))
        await asyncio.wait([task])

        if self.action == "delete":
            await self.delete_capture(parent)

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

def _setup_logging(log_str: str = "") -> None:
    log_level = LOGGING_LEVEL
    if log_str == "DEBUG":
        log_level = logging.DEBUG
    elif log_str == "INFO":
        log_level = logging.INFO
    elif log_str == "WARN":
        log_level = logging.WARN
    elif log_str == "ERROR":
        log_level = logging.ERROR
    logging.basicConfig(level=log_level)

    debug_formatter = logging.Formatter(
        "%(asctime)s %(name)s:%(levelname)s: %(message)s", datefmt="%Y-%m-%d %H:%M:%S"
    )
    debug_file_handler = logging.FileHandler("image_downloader_total.log")
    debug_file_handler.setFormatter(debug_formatter)
    logging.getLogger().addHandler(debug_file_handler)

    # This formatter seems to be added already, ignore it.
    # debug_print_handler = logging.StreamHandler(sys.stdout)
    # debug_print_handler.setFormatter(debug_formatter)
    # logging.getLogger().addHandler(debug_print_handler)


def parse_args() -> None:
    parser = argparse.ArgumentParser(
        prog="Image downloader total", description="Download images from the cameras")

    cam1 = parser.add_mutually_exclusive_group()
    cam1.add_argument("--cam1", action="store_true", dest="cam1", default=False)
    cam1.add_argument("--no-cam1", action="store_false", dest="cam1")

    cam2 = parser.add_mutually_exclusive_group()
    cam2.add_argument("--cam2", action="store_true", dest="cam2", default=False)
    cam2.add_argument("--no-cam2", action="store_false", dest="cam2")

    parser.add_argument("--cam1-filter", type=str, default="")
    parser.add_argument("--cam2-filter", type=str, default="")

    parser.add_argument("--cam1-delete", action="store_true", help="Instead of downloading the image from camera 1, delete it")
    parser.add_argument("--cam2-delete", action="store_true", help="Instead of downloading the image from camera 2, delete it")

    parser.add_argument("--logging-level", default="INFO", choices=("DEBUG", "INFO", "WARN", "ERROR"))

    return parser.parse_args()


async def main() -> None:
    args = parse_args()
    # print(args)
    # Initialize the loggers
    _setup_logging(args.logging_level)
    _logger = logging.getLogger("main")

    # Initialize the cameras
    cams = []
    if args.cam1 or args.cam1_filter:
        cam1 = Camera("cam1", "http://192.168.1.83:80", name_filter=args.cam1_filter, action="delete" if args.cam1_delete else "download")
        cams.append(cam1)
    if args.cam2 or args.cam2_filter:
        cam2 = Camera("cam2", "http://192.168.3.83:80", name_filter=args.cam2_filter, action="delete" if args.cam2_delete else "download")
        cams.append(cam2)

    # Get the file tree from each camera
    trees = []
    for cam in cams:
        tree = await cam.get_file_tree()
        trees.append((cam, tree))

    # Download the images from each camera.
    coros = (cam.walk_tree(tree) for cam, tree in trees)
    task = asyncio.gather(*coros)
    await asyncio.wait([task])


if __name__ == "__main__":
    try:
        # Run the main function.
        asyncio.run(main())
    except RuntimeError:
        # Loop already running, just push a Future.
        asyncio.ensure_future(main())
