import serial
import time
import os


porta_seriale = '/dev/ttyUSB1'
baud_rate = 9600  # Impostare questo tasso di baud a quello utilizzato dal sensore


percorso_scrivania = os.path.join(os.path.expanduser("~"), "Scrivania")
directory_output = os.path.join(percorso_scrivania, "dati_sensore")
if not os.path.exists(directory_output):
    os.makedirs(directory_output)
file_output = os.path.join(directory_output, "output_sensore.txt")


ser = serial.Serial(porta_seriale, baud_rate)

def parse_data(data):
    if data.startswith("$ISADS"):
        # Esempio di stringa: $ISADS,000.000,M,33.1,C*73
        parts = data.split(',')
        distanza = parts[1] + ' ' + parts[2]
        temperatura = parts[3] + ' ' + parts[4].split('*')[0]
        return f"Distanza: {distanza}, Temperatura: {temperatura}"
    elif data.startswith("$ISHPR"):
        # Esempio di stringa: $ISHPR,243.4,+01.3,+001.1*61
        parts = data.split(',')
        cap = parts[1]
        beccheggio = parts[2]
        rollio = parts[3].split('*')[0]
        return f"Cap: {cap}°, Beccheggio: {beccheggio}°, Rollio: {rollio}°"
    else:
        return "Dati sconosciuti"

try:
    with open(file_output, 'a') as file:
        while True:

            data = ser.readline().decode('ascii').strip()
            timestamp = time.strftime('%Y-%m-%d %H:%M:%S')
            dati_formattati = parse_data(data)
            print(f"{timestamp} - {dati_formattati}")
            file.write(f"{timestamp} - {dati_formattati}\n")
            time.sleep(1)
except KeyboardInterrupt:
    print("Interruzione del script.")
finally:
    ser.close()
