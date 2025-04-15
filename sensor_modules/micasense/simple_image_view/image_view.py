#!/usr/bin/env python3
# -*- coding: utf-8 -*-

if True:
    import sys

    sys.path.append("../imageprocessing/")

import asyncio
import csv
from pathlib import Path, PurePosixPath

import aiofiles
import aiohttp

# mypy: ignore-errors
# import micasense.capture as capture

BASE_IMAGE_STORAGE = Path("images/")
EVENT_SOURCE = Path("../camera_capture/events.csv")


async def download_image(
    session: aiohttp.ClientSession, url: str, destination: str
) -> None:
    async with session.get(url) as resp:
        async with aiofiles.open(destination, "wb") as file:
            content = await resp.read()
            await file.write(content)
        print("File", destination, "saved with success")


async def download_image_from_events(camera_map: dict[str, str]) -> None:
    camera_image_index: dict[str, int] = {key: 0 for key in camera_map}

    async with aiohttp.ClientSession() as session:
        async with aiofiles.open(EVENT_SOURCE, "r") as file:
            async for line in file:
                csv_record = next(csv.reader([line]))
                await _save_event_record(
                    session, camera_map, camera_image_index, list(csv_record)
                )


async def _save_event_record(
    session: aiohttp.ClientSession,
    camera_map: dict[str, str],
    camera_image_index: dict[str, int],
    csv_record: list[str],
) -> None:
    camera = csv_record[0]
    if camera not in camera_map:
        return

    for image_path in csv_record[2:]:
        image_filename = PurePosixPath(image_path).stem
        image_index = image_filename.split("_")[-1]

        out_path = "images/{:s}_{:d}_{:s}.tiff".format(
            camera, camera_image_index[camera], image_index
        )
        camera_image_index[camera] += 1

        await download_image(session, camera_map[camera] + image_path, out_path)


async def main():
    Path("images").mkdir(exist_ok=True)
    await download_image_from_events(
        {"cam1": "http://192.168.1.83", "cam2": "http://192.168.2.83"}
    )
    # await download_last_capture("http://192.168.1.83")
    # images = [str(x) for x in BASE_IMAGE_STORAGE.glob("*")]
    # if images:
    #     cap = capture.Capture.from_filelist(images)
    #     cap.plot_radiance()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except RuntimeError:
        # Loop already running, just push a Future.
        asyncio.ensure_future(main())
