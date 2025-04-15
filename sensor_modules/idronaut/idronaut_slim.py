import re
import serial
import signal
import sys
import threading
from datetime import datetime
import urllib3
import influxdb_client
from influxdb_client.client.write_api import SYNCHRONOUS

DEVICE_NAME = "IDRONAUT OCEAN SEVEN 310"
SERIAL_PORT = "/dev/ttyUSB0"
# SERIAL_PORT="COM18"
BAUDRATE = 38400
shutdown = False
dataHarvesting = False
idronaut_thread = None

# Mutex per evitare race condition sulle strutture dati.
serial_data_lock = threading.Lock()


def filter(s):
    return s.startswith("$GNRMC") or s.startswith("$GPRMC")


def tokenize(line):
    return line.split(",")

def parse_data_string(line):
    # use regex to extract all fnumbers
    numbers = re.findall(r"[-+]?\d*\.?\d+", line)
    
    # mlast two numbers, as idronaut splits the ph
    if len(numbers) > 7:
        numbers[-2] += numbers[-1]
        del numbers[-1]
    
    # conversion
    vector = [float(num) for num in numbers]
    
    # sanity check
    if len(vector) != 7:
        raise ValueError("Parsed vector does not have exactly 7 elements")
    
    return vector


def send_command(s, string):
    global dataHarvesting
    global last_position
    global shutdown
    if s.open and (not shutdown or string == "SO\n"):
        if string == "CT\r\n":
            print("Starting data harvesting")
            dataHarvesting = True
        else:
            dataHarvesting = False

        if string == "\r\n":
            print("Sending newline")
        else:
            print("Sending command: " + string)

        if string=='SO\n' or string=='AZ\n':
            print('Executing '+string)
            if s.write_line(string):
                line=s.read_line()
                while not line.startswith('ER 000'):
                    print('Waiting for ER 000... ')
                    line=s.read_line()
            else:
                print('ERROR')
        else:
            print("Executing " + string)
            if s.write_line(string):
                line = s.read_line()
                while line != "" and not shutdown:
                    print(line)
                    if dataHarvesting:
                        idro_data = parse_data_string(line)
                        s._send_to_influx(idro_data)
                    line = s.read_line()
            else:
                print("ERROR")


class serial_port:
    def __init__(self, label, port_number, baudrate):
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
        self.influx_org = "SailingLab"
        self.client = influxdb_client.InfluxDBClient(
            url="http://localhost:8086",
            token="M6a4pxEjNKvmevvo4mXddIzJQTdRf9xkuxffkDuCoExREgiGvpy6sOc1bhGYi9-La6QMw_PVsDf5fXakw5brLg==",
            org=self.influx_org,
        )
        self.write_api = self.client.write_api(write_options=SYNCHRONOUS)

    def open_connection(self):
        if not self.open:
            print("(" + self.label + ") opening connection")
            try:
                self.port.open()
                self.open = True
                return True
            except Exception:
                self.open = False
                return False

    def read_line(self):
        try:
            line = str(self.port.readline().decode("ascii")).strip()
        except Exception:
            line = ""
        return line

    def write_line(self, string):
        no_error = True
        try:
            _line = str(self.port.write(string.encode()))
            # self.port.flush()
        except Exception:
            no_error = False
        return no_error

    def close_connection(self):
        if self.open:
            print("(" + self.label + ") closing connection")
            self.port.close()
            self.open = False

    def __del__(self):
        if self.open:
            self.port.close()
            self.open = False

    def _send_to_influx(self, data) -> bool:
        try:
            p = influxdb_client.Point(
                "idronaut_data"
            ).field("pressure", data[0]
            ).field("temperature", data[1]
            ).field("conductivity", data[2]
            ).field("salinity", data[3]
            ).field("oxygen_percentage", data[4]
            ).field("oxygen_ppm", data[5]
            ).field("ph", data[6])
            self.write_api.write(bucket="asv_data", org=self.influx_org, record=p)
            return True
        except (ConnectionResetError, OSError, urllib3.exceptions.ProtocolError, urllib3.exceptions.NewConnectionError) as ex:
            self.logger.error("Failed to send coordinates to influx. Reason %s", ex)
            return False


def signal_handler(signal, frame):
    global shutdown
    global s_port
    global dataHarvesting
    global idronaut_thread
    global SERVICE_PORT
    shutdown = True
    print("You pressed Ctrl+C!\nExiting...")

    if s_port.open:
        if dataHarvesting:
            print("Stopping data harvesting")
            dataHarvesting = False
            s_port.client.close()
            if idronaut_thread is not None:
                idronaut_thread.join()
                print("signal_handler: IDRONAUT thread exited!")
                idronaut_thread = None


def idronaut_reader(s_port):
    # Awake probe sending newline
    send_command(s_port, "\r\n")
    # Let's try to revert to verbose mode (in case we aren't)
    send_command(s_port, "VT\r\n")
    # Switch to non-verbose mode
    send_command(s_port, "5\r\n")

    now = datetime.now()
    timeStr=now.strftime("%d-%m-%Y %H:%M:%S")
    set_current_time = "TS " + timeStr + "\r\n"
    print(set_current_time)
    send_command(s_port, set_current_time)
    # start automatic pressure calibration
    send_command(s_port,'AZ\n')
    # start Continous Time data gathering
    send_command(s_port,'CT\r\n')
    print('Exiting from CT')
    send_command(s_port,'SO\n')     


signal.signal(signal.SIGTERM, signal_handler)
print("Press Ctrl+C to stop and exit!")

if not shutdown:
    s_port = serial_port(DEVICE_NAME, SERIAL_PORT, BAUDRATE)
    if s_port.open_connection():
        idronaut_thread = threading.Thread(
            target=idronaut_reader,
            args=(
                s_port,
            ),
        )
        idronaut_thread.start()
        print("IDRONAUT thread started!")
    else:
        shutdown = True

if idronaut_thread is not None:
    idronaut_thread.join()
    print("IDRONAUT thread exited!")

s_port.close_connection()

sys.exit(0)
