#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import asyncio
import logging
import operator
import platform
import subprocess
from datetime import datetime
from functools import reduce
from typing import Any, Optional, Union

import influxdb_client
import serial
import urllib3
from influxdb_client.client.write_api import SYNCHRONOUS

from brping import Ping1D


"""
The gps device name.
"""
GPS_DEVICE_NAME = "Kendau GPS"

"""
The gps device serial port.
"""
GPS_SERIAL_PORT = "/dev/gps"

"""
The gps device baud rate.
"""
GPS_BAUDRATE = 9600

"""
The pingsonar device address.
"""
PING_ADDR = "192.168.2.2"

"""
The pingsonar device port.
"""
PIGN_PORT = 9090

"""
The global logging level. This controls what is logged.
"""
LOGGING_LEVEL = logging.INFO


# Adapted from idronaut.py
class SerialPort:
    label: str
    port: serial.Serial
    open: bool
    debug_logger: logging.Logger

    @staticmethod
    def parse_payload(line: str) -> str:
        return line[1:] if line.startswith("$") else line

    @staticmethod
    def tokenize(line: str) -> list[str]:
        return line.split(",")

    @staticmethod
    def validate_checksum(
        payload: str, checksum: str, checksum_prefix: str = "*"
    ) -> bool:
        """Validate the NMEA checksum

        Based on: https://forum.arduino.cc/t/nmea-checksums-explained/1046083

        :return: True if the payload pass the checksum test, False otherwise.
        """
        if not checksum.startswith(checksum_prefix):
            # If the prefix is not correct, it's very difficult that the other bytes are correct.
            return False

        # Add the constant bytes of the prefix to the payload, except the last char, which is '*'.
        payload += checksum_prefix[:-1]
        # Remove the constant prefix from the checksum.
        checksum = checksum[len(checksum_prefix) :]

        # Based on https://gist.github.com/MattWoodhead/0bc2b3066796e19a3a350689b43b50ab
        calculated_checksum = reduce(operator.xor, (ord(s) for s in payload), 0)
        if int(checksum, base=16) != calculated_checksum:
            return False

        return True

    def __init__(self, label: str, port_number: str, baudrate: int = 9600):
        super().__init__()
        self.label = label
        self.port = serial.Serial()
        self.port.port = port_number
        self.port.baudrate = baudrate
        self.open = False
        self.port.bytesize = serial.EIGHTBITS
        self.port.parity = serial.PARITY_NONE
        self.port.stopbits = serial.STOPBITS_ONE
        self.port.timeout = 5  # Leggeremo una linea per volta: il timeout (in secondi) serve a non rimanere bloccati in caso di assenza di dati dal dispositivo
        self.port.xonxoff = False  # disabilita il flusso di controllo software
        self.port.rtscts = False  # disabilita il flusso di controllo hardware (RTS/CTS)
        self.port.dsrdtr = False  # disabilita il flusso di controllo hardware (DSR/DTR)

        self.debug_logger = logging.getLogger(label + " serial")

    def open_connection(self) -> bool:
        if not self.open:
            self.debug_logger.info("Opening connection")
            try:
                self.port.open()
                self.open = True
                self.debug_logger.debug("Serial port successfully opened")
                return True
            except Exception as ex:
                self.debug_logger.error(
                    "Failed to open serial port %s", self.port.port, exc_info=ex
                )
                self.open = False
                return False
        return True

    def read_line(self) -> str:
        try:
            line_bytes = self.port.readline()
            line = str(line_bytes.decode("ascii")).strip("\n\r\t")
        except Exception as ex:
            self.debug_logger.debug(
                "Failed to read data from serial '%s'.", self.port.port, exc_info=ex
            )
            line = ""
        return line

    def write_line(self, string) -> bool:
        no_error = True
        try:
            _line = str(self.port.write(string.encode()))
            # self.port.flush()
        except Exception:
            no_error = False
        return no_error

    def close_connection(self) -> None:
        if self.open:
            self.debug_logger.info("Closing connection")
            self.port.close()
            self.open = False

    def __del__(self) -> None:
        if self.open:
            self.port.close()
            self.open = False


class GPS:
    serial_port: SerialPort

    ping_sonar = Ping1D()
    ping_data = -1

    gps_data: Union[tuple[Any, Any], tuple] = ()
    date_updated: bool

    influx_org: str
    write_api: influxdb_client.client.write_api.WriteApi

    # event_logger: logging.Logger
    debug_logger: logging.Logger

    def __init__(self, serial_port: SerialPort):
        super().__init__()
        self.serial_port = serial_port

        self._setup_logging()
        self.date_updated = False

        self.influx_org = "SailingLab"
        client = influxdb_client.InfluxDBClient(
            url="http://localhost:8086",
            token="ijL6ry3VP0Hm5nAvP-wvHouC1l3ysIWty-VWCPgF7Bz-aKt-4Oi9zFMV_t8UkVnQSVwdxlRpdKjbAuPxx9umsA==",
            org=self.influx_org,
        )
        self.write_api = client.write_api(write_options=SYNCHRONOUS)

        self.ping_sonar.connect_udp(PING_ADDR, PIGN_PORT)
        if self.ping_sonar.initialize() is False:
            self.debug_logger.debug("Failed to initialize ping sonar")

    def _setup_logging(self) -> None:
        # Initialize the event logging file.
        # self.event_logger = logging.getLogger("gps")

        # event_handler = logging.FileHandler("events.csv")

        # self.event_logger.addHandler(event_handler)
        # self.event_logger.propagate = False

        # Initialize the debug logger
        self.debug_logger = logging.getLogger("depthmeter_debug")

    async def do_loop_cycle(self) -> None:
        loop = asyncio.get_event_loop()
        result: Optional[str] = await loop.run_in_executor(
            None, self.serial_port.read_line
        )
        if result is None or result == "":
            return
        elif self._process_gps_data(result):
            pass
        else:
            # self.debug_logger.warning("Failed to parse string '%s'", result)
            return

        await self._flush_data()

    def _process_gps_data(self, line: str) -> bool:
        if line.startswith("$GNRMC") or line.startswith("$GPRMC"):
            self.debug_logger.debug("Process gps data sentence '%s'", line)
            # Get the real payload from the NMEA sentence
            payload = SerialPort.parse_payload(line)

            # Get the checksum.
            # The last 3 bytes are always the checksum
            checksum = payload[-3:]

            # Remove the checksum from the payload string
            payload = payload[:-3]
            if not SerialPort.validate_checksum(payload, checksum):
                self.debug_logger.warning(
                    "Sentence '%s' is corrupted. NMEA checksum verification failed.",
                    line,
                )
                return True

            # Split payload into tokens.
            tokens = SerialPort.tokenize(payload)
            if len(tokens) != 13:
                self.debug_logger.warning(
                    "Sentence '%s' has invalid format. Expected 13 tokens, got %d",
                    line,
                    len(tokens),
                )
                return True

            # Get sensor data from tokens
            (
                _,
                utc_time,
                a_char,
                lat,
                latdir,
                lon,
                londir,
                sog,
                cog,
                utc_date,
                _,
                _,
                _,
            ) = tokens

            dt = datetime.strptime(f"{utc_date} {utc_time}", "%d%m%y %H%M%S.%f")

            if a_char != "A":
                # Invalid data, ignore it
                return True

            # Store sensor data.
            if self.gps_data:
                self.debug_logger.debug("GPS data was not consumed, override it.")
            self.gps_data = (
                dt,
                float(lat),
                latdir,
                float(lon),
                londir,
                float(sog),
                float(cog),
            )

            data = self.ping_sonar.get_distance()
            if data:
                self.ping_data = data["distance"]
            else:
                self.ping_data = -1000
                self.debug_logger.debug("Failed to get ping sonar distance data")

            return True
        return False

    async def _flush_data(self) -> None:
        """Add a line to the event log, if gps data is available."""
        if not self.gps_data:
            return

        # Create CSV columns using a format string.
        # Add a column for each field in depth and inertial data.
        # Remove the last ',' as an empty column is useless.
        fmt_string = f"{'%s,' * len(self.gps_data)}"[:-1]

        self.debug_logger.debug(f"Log data with the format: '{fmt_string}'")
        if LOGGING_LEVEL >= logging.INFO:
            # Print data to show that it's working.
            print(*self.gps_data, sep=", ")

        loop = asyncio.get_event_loop()
        if not self.date_updated:
            await loop.run_in_executor(None, self._update_os_date, self.gps_data[0])
        if not self._send_to_influx():
            return
        self.gps_data = ()

    def _send_to_influx(self) -> bool:
        try:
            p = influxdb_client.Point(
                "gps_data2"
            ).field("latitude", self.gps_data[1]
            ).field("latitude_dir", self.gps_data[2]
            ).field("longitude", self.gps_data[3]
            ).field("longitude_field", self.gps_data[4]
            ).field("sog", self.gps_data[5]
            ).field("cog", self.gps_data[6]
            ).field("depth", self.ping_data/1000)
            self.write_api.write(bucket="asv_data", org=self.influx_org, record=p)
            return True
        except (ConnectionResetError, OSError, urllib3.exceptions.ProtocolError, urllib3.exceptions.NewConnectionError) as ex:
            self.logger.error("Failed to send coordinates to influx. Reason %s", ex)
            return False

    def _update_os_date(self, date: datetime) -> None:
        if platform.system() == "Linux":
            try:
                out = subprocess.run(["sudo", "date", "-s", date.isoformat()])
                self.debug_logger.debug("Successful date command. %s", out)
            except Exception as ex:
                self.debug_logger.error("Failed to execute date update", exc_info=ex)
        elif platform.system() == "Windows":
            try:
                import win32api
            except ImportError:
                self.debug_logger.error("pywin32/win32api module is missing")
                return
            win32api.SetSystemTime(
                date.year,
                date.month,
                0,
                date.day,
                date.hour,
                date.minute,
                date.second,
                0,
            )
        else:
            self.debug_logger.warning("Unrecognized platform %s", platform.system())
            return

        self.debug_logger.info("System datetime updated")
        self.date_updated = True


def _setup_logging() -> None:
    logging.basicConfig(level=LOGGING_LEVEL)

    debug_formatter = logging.Formatter(
        "%(asctime)s %(name)s:%(levelname)s: %(message)s", datefmt="%Y-%m-%d %H:%M:%S"
    )
    debug_file_handler = logging.FileHandler("gps_logger.log")
    debug_file_handler.setFormatter(debug_formatter)
    logging.getLogger().addHandler(debug_file_handler)


async def main() -> None:
    # Initialize the loggers
    _setup_logging()
    logger = logging.getLogger("main")

    # Initialize the serial port.
    gps_serial = SerialPort(GPS_DEVICE_NAME, GPS_SERIAL_PORT, GPS_BAUDRATE)
    gps_serial.open_connection()

    gps = GPS(gps_serial)

    # Start the log loop
    loop = asyncio.get_event_loop()
    running = True
    while running and loop.is_running():
        try:
            # All the expensive processing is here.
            await gps.do_loop_cycle()
        except Exception as ex:
            logger.error("Logging fails", exc_info=ex)
            # DEBUG: Abort on error for debugging.
            # running = False
        except BaseException:
            logger.debug("Request to stop")
            running = False
    gps_serial.close_connection()


if __name__ == "__main__":
    subprocess.run(["bash", "./stop-NTP.sh"])
    try:
        # Run the main function.
        asyncio.run(main())
    except RuntimeError:
        # Loop already running, just push a Future.
        asyncio.ensure_future(main())
