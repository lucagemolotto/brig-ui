import serial
import signal
import sys
import os
import time
from datetime import datetime
import threading
import socket

DEVICE_NAME="IDRONAUT OCEAN SEVEN 310"
SERIAL_PORT="/dev/ttyUSB0"
#SERIAL_PORT="COM18"
BAUDRATE=38400
GPS_DEVICE_NAME="Kendau GPS"
GPS_SERIAL_PORT="/dev/ttyUSB2"
#GPS_SERIAL_PORT="COM17"
GPS_BAUDRATE=9600
shutdown=False
dataHarvesting=False
logFileName=""
logFile=None
gps_thread=None
idronaut_thread=None
# Socket Server
HOST = '' # Symbolic name meaning all available interfaces
SERVICE_PORT=12345
server_thread=None

# Una classe per rappresentare le coordinate GPS di un oggetto
class coordinates:
	def __init__(self,label,lat,latdir,lon,londir,sog,cog,date_time):
		self.label=label
		self.lat=lat
		self.latdir=latdir
		self.lon=lon
		self.londir=londir
		self.sog=sog
		self.cog=cog
		self.date_time=date_time

	def print_json(self):
		return '{"entity_name": "'+self.label+\
		        '", "lat": "'+str(self.lat)+\
		        '", "latdir": "'+str(self.latdir)+\
		        '", "lon": "'+str(self.lon)+\
		        '", "londir": "'+str(self.londir)+\
		        '", "sog": "'+str(self.sog)+\
		        '", "cog": "'+str(self.cog)+\
		        '", "date_time": "'+str(self.date_time)+'"}'

# Variabile per memorizzare l'ultima posizione valida dell'oggetto tracciato.
last_position = coordinates(DEVICE_NAME,'unknown','unknown','unknown','unknown','unknown','unknown','unknown')

# Mutex per evitare race condition sulle strutture dati.
serial_data_lock = threading.Lock()

def filter(s):
	return s.startswith('$GNRMC') or s.startswith('$GPRMC')

def tokenize(line):
	return line.split(',')

def gps_init(s):
	global shutdown
	if not s.open_connection():
		shutdown=True
	valid_flag=False
	while not valid_flag and not shutdown:
		new_data=s.read_line()
		#print('gps_init: '+new_data)
		if len(new_data)>0 and filter(new_data):
			serial_data_lock.acquire()
			fields=tokenize(new_data)
			if fields[2]=='A' : # se i dati sono validi
				last_position.lat=fields[3]
				last_position.latdir=fields[4]
				last_position.lon=fields[5]
				last_position.londir=fields[6]
				last_position.sog=fields[7]
				last_position.cog=fields[8]
				last_position.date_time=fields[9]+' '+fields[1]
				valid_flag=True
			serial_data_lock.release()
	s.close_connection()

def gps_reader(s):
	global shutdown
	if not s.open_connection():
		shutdown=True
	while not shutdown:
		new_data=s.read_line()
		#print('gps_reader: '+new_data)
		if len(new_data)>0 and filter(new_data):
			serial_data_lock.acquire()
			fields=tokenize(new_data)
			if fields[2]=='A' : # se i dati sono validi
				last_position.lat=fields[3]
				last_position.latdir=fields[4]
				last_position.lon=fields[5]
				last_position.londir=fields[6]
				last_position.sog=fields[7]
				last_position.cog=fields[8]
				last_position.date_time=fields[9]+' '+fields[1]
			serial_data_lock.release()
	s.close_connection()

def send_command(s,string):
	global dataHarvesting
	global logFileName
	global logFile
	global last_position
	global shutdown
	if s.open and (not shutdown or string=='SO\n'):
		if string=='CT\r\n':
			print("Starting data harvesting")
			dataHarvesting=True
			logFile=open(logFileName,"a")
		else:
			dataHarvesting=False

		if string=='\r\n':
			print('Sending newline')
		else:
			print('Sending command: '+string)

		if string=='SO\n':
			print('Executing SO')
			if s.write_line(string):
				line=s.read_line()
				while not line.startswith('ER 000'):
					print('Waiting for ER 000... ')
					line=s.read_line()
			else:
				print('ERROR')		
		else:
			print('Executing '+string)
			if s.write_line(string):
				line=s.read_line()
				while line!='' and not shutdown:
					print(line)
					if dataHarvesting:
						serial_data_lock.acquire()
						logFile.write(last_position.date_time+ ','+ last_position.lat+','+ last_position.latdir+','+ last_position.lon+','+ last_position.londir+','+ last_position.sog+','+ last_position.cog+'#'+line+'\n')
						serial_data_lock.release()
						logFile.flush()
					line=s.read_line()
			else:
				print('ERROR')

class serial_port:
	def __init__(self,label,port_number,baudrate):
		self.label = label
		self.port = serial.Serial()
		self.port.port = port_number
		self.port.baudrate=baudrate
		self.open=False
		self.port.bytesize=serial.EIGHTBITS
		self.port.parity=serial.PARITY_NONE
		self.port.stopbits=serial.STOPBITS_ONE
		self.port.timeout=5          # Leggeremo una linea per volta: il timeout (in secondi) serve a non rimanere bloccati in caso di assenza di dati dal dispositivo
		self.port.xonxoff = False    # disabilita il flusso di controllo software
		self.port.rtscts = False     # disabilita il flusso di controllo hardware (RTS/CTS)
		self.port.dsrdtr = False     # disabilita il flusso di controllo hardware (DSR/DTR)

	def open_connection(self):
		if not self.open:
			print('('+self.label+') opening connection')
			try:
				self.port.open()
				self.open = True
				return True
			except:
				self.open = False
				return False

	def read_line(self):
		try:
			line = str(self.port.readline().decode('ascii')).strip()
		except:
			line=''
		return line

	def write_line(self, string):
		no_error=True
		try:
			line = str(self.port.write(string.encode()))
			#self.port.flush()
		except:
			no_error=False
		return no_error

	def close_connection(self):
		if self.open:
			print('('+self.label+') closing connection')
			self.port.close()
			self.open = False

	def __del__(self):
		if self.open:
			self.port.close()
			self.open = False

def signal_handler(signal, frame):
	global shutdown
	global s_port
	global dataHarvesting
	global logFile
	global idronaut_thread
	global SERVICE_PORT
	shutdown = True
	print('You pressed Ctrl+C!\nExiting...')
	try: # connessione dummy per svegliare il server, se è rimasto bloccato in stato di accept
		s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
		s.connect(('localhost',SERVICE_PORT))
		bytes_recv=''
		while bytes_recv!=msg:
			bytes_recv = bytes_recv+s.recv(4096).decode()
			if not bytes_recv:
				break
	except:
		pass

	if s_port.open:
	  if dataHarvesting:
	  	print('Stopping data harvesting')
	  	dataHarvesting=False
	  	if not idronaut_thread==None:
	  		idronaut_thread.join()
	  		print('signal_handler: IDRONAUT thread exited!')
	  		idronaut_thread=None

def change_time(s):
    return
	global last_position
	day=last_position.date_time[0:2]
	month=last_position.date_time[2:4]
	year='20'+last_position.date_time[4:6]
	hour=last_position.date_time[7:9]
	minute=last_position.date_time[9:11]
	second=last_position.date_time[11:13]
	if s == 1:
		os.system('date -s "'+year+'-'+month+'-'+day+ ' '+hour+':'+minute+':'+second+'"')
		print("***Date-time updated!***")
	elif s == 2:
		try:
			import win32api
		except ImportError:
			print('change_time: pywin32/win32api module is missing!')
			sys.exit(1)
		win32api.SetSystemTime(int(year), int(month), 0, int(day), int(hour), int(minute), int(second), 0)
	else:
		print('change_time: wrong parameter!')

def check_os():
  if sys.platform=='linux':
    print('Linux system detected!')
    change_time(1)
  if sys.platform=='linux2':
    print('Raspberry Pi system detected!')
    change_time(1)
  elif sys.platform=='win32':
    print('Win32 system detected!')
    change_time(2)
  else:print('unknown system')

def idronaut_reader(s_port,timeStr):
	# Awake probe sending newline
	send_command(s_port,'\r\n')
	# Let's try to revert to verbose mode (in case we aren't)
	send_command(s_port,'VT\r\n')
	# Switch to non-verbose mode
	send_command(s_port,'5\r\n')

	#now = datetime.now()
	#timeStr=now.strftime("%d-%m-%Y %H:%M:%S")
	set_current_time = "TS "+timeStr+"\r\n"
	print(set_current_time)
	send_command(s_port,set_current_time)

	send_command(s_port,'CT\r\n')
	print('Exiting from CT')
	send_command(s_port,'SO\n')				

#Function for handling connections. This will be used to create threads
def handleClient(conn):
	global shutdown
	global last_position
	#infinite loop so that function do not terminate and thread do not end.
	if not shutdown:
		try:
			print("sending time and GPS coordinates...")
			conn.sendall((last_position.print_json()).encode())
		except:
			print('Error in client request: ', sys.exc_info()[0])

	#came out of loop
	print('Closing client connection')
	conn.close()

def server(service_socket):
	global HOST
	global SERVICE_PORT
	service_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
	print('Service Socket created')

	#Bind socket to local host and service port
	try:
		service_socket.bind((HOST, SERVICE_PORT))
	except socket.error as msg:
		print('(Service port) Bind failed. Error Code : ' + str(msg[0]) + ' Message ' + msg[1])
		sys.exit(1)
     
	print('(Service port) Socket bind complete')
 
	#Start listening on service socket
	service_socket.listen(10)
	print('Service Socket now listening')
 
	#now keep talking with the client
	while not shutdown:
		#wait to accept a connection - blocking call
		conn, addr = service_socket.accept()
		print('Connected with ' + addr[0] + ':' + str(addr[1]))
     
		#start new thread takes 1st argument as a function name to be run, second is the tuple of arguments to the function.
		client_thread=threading.Thread(None,handleClient,None,(conn,))
		client_thread.start()
 
	service_socket.close()

signal.signal(signal.SIGINT, signal_handler)
print('Press Ctrl+C to stop and exit!')

if not shutdown:
	gps_port=serial_port(GPS_DEVICE_NAME,GPS_SERIAL_PORT,GPS_BAUDRATE)
	s_port=serial_port(DEVICE_NAME,SERIAL_PORT,BAUDRATE)

	gps_init(gps_port)

if not shutdown:
	check_os()

if not shutdown:
	serial_data_lock.acquire()
	timeStr=last_position.date_time[0:2]+ '-'+last_position.date_time[2:4]+'-20'+last_position.date_time[4:6]
	timeStr+=" "+last_position.date_time[7:9]+':'+last_position.date_time[9:11]+':'+last_position.date_time[11:13]
	logFileName="log"+timeStr.replace(':','_')+".txt"
	serial_data_lock.release()

if not shutdown:
	gps_thread=threading.Thread(target=gps_reader,args=(gps_port,))
	gps_thread.start()
	print('GPS thread started!')
	service_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
	server_thread=threading.Thread(None,server,None,(service_socket,))
	server_thread.start()

if not shutdown:
	if s_port.open_connection():
		idronaut_thread=threading.Thread(target=idronaut_reader, args=(s_port, timeStr,))
		idronaut_thread.start()
		print('IDRONAUT thread started!')
	else:
		shutdown=True

if not gps_thread==None:
	gps_thread.join()
	print('GPS thread exited!')

if not server_thread==None:
	server_thread.join()
	print('Server thread exited!')

if not idronaut_thread==None:
	idronaut_thread.join()
	print('IDRONAUT thread exited!')

if not logFile==None:
	logFile.close()

s_port.close_connection()

sys.exit(0)
