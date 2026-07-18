# GRAPHICAL APP GOES HERE

import subprocess
import ctypes
import tkinter as tk


result = subprocess.run(
    ["./main"],
    input="BatScript",
    text=True,
    capture_output=True
)

print(result.stdout)


vulpin_lib = ctypes.CDLL('./libvulpin.so')
# specify the args for all upcoming C functions.
vulpin_lib.run_vulpin.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_int]

def on_run_button_click():
    filename = "test.vul" # FIX SO IT GETS FROM UI
    
    # Create a buffer for the C code to fill
    buffer = ctypes.create_string_buffer(2048)
    
    # Call the C function
    vulpin_lib.run_vulpin(filename.encode('utf-8'), buffer, 2048)
    
    # Put the result into Tkinter widget
    output = buffer.value.decode('utf-8')
    console.insert("end", output)




# STEFUN FIX THIS WITH RUNTIME
