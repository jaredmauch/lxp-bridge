#!/usr/bin/env python3

import base64
import struct
import json
import argparse
from typing import Dict, Any

def load_register_definitions() -> Dict[int, Any]:
    """Load register definitions from JSON file."""
    with open('eg4_registers.json', 'r') as f:
        data = json.load(f)
        # Create a dictionary mapping register numbers to their definitions
        registers = {}
        for reg_type in data['registers']:
            if 'register_map' in reg_type:
                for reg in reg_type['register_map']:
                    if 'register_number' in reg:
                        registers[reg['register_number']] = reg
        print(f"Loaded {len(registers)} register definitions")
        return registers

def decode_value_frame(value_frame: str) -> bytes:
    """Decode base64 valueFrame string."""
    data = base64.b64decode(value_frame)
    print(f"Decoded valueFrame length: {len(data)} bytes")
    
    # Print first few bytes for debugging
    print("\nFirst 32 bytes of data:")
    print("Hex: ", end='')
    for i, b in enumerate(data[:32]):
        print(f"{b:02X} ", end='')
        if (i + 1) % 16 == 0:
            print("\nASCII:", end='')
            for c in data[i-15:i+1]:
                if 32 <= c <= 126:  # Printable ASCII
                    print(chr(c), end='')
                else:
                    print('.', end='')
            print("\nHex: ", end='')
    print("\n")
    
    return data

def parse_register_value(data: bytes, reg_def: Dict[str, Any], offset: int) -> Any:
    """Parse a register value according to its definition."""
    if 'num_values' in reg_def:
        # Handle multi-value registers
        values = {}
        for value_map in reg_def['value_map']:
            if value_map['value_unit'] == 'bit':
                # Extract bits from the value
                mask = (1 << value_map['value_size']) - 1
                value = (int.from_bytes(data[offset:offset+2], 'big') >> value_map['value_location']) & mask
                if 'value_map' in value_map:
                    value = value_map['value_map'].get(str(value), value)
                values[value_map['shortname']] = value
            elif value_map['value_unit'] == 'byte':
                # Extract bytes from the value
                value = data[offset + value_map['value_location']]
                if 'value_map' in value_map:
                    value = value_map['value_map'].get(str(value), value)
                values[value_map['shortname']] = value
        return values
    else:
        # Handle single-value registers
        datatype = reg_def.get('datatype', 'uint16')
        if datatype == 'uint16':
            value = struct.unpack('>H', data[offset:offset+2])[0]
        elif datatype == 'float':
            value = struct.unpack('>f', data[offset:offset+4])[0]
        elif datatype == 'uint8':
            value = data[offset]
        else:
            value = struct.unpack('>H', data[offset:offset+2])[0]

        # Apply unit scale if specified
        if 'unit_scale' in reg_def:
            value *= float(reg_def['unit_scale'])

        # Apply value mapping if specified
        if 'value_map' in reg_def:
            value = reg_def['value_map'].get(str(value), value)

        return value

def process_portal_file(filename: str, registers: Dict[int, Any]):
    """Process a single portal file and print its register values."""
    print(f"\nProcessing file: {filename}")
    print("=" * 50)
    
    # Read valueFrame and startRegister from portal file
    with open(filename, 'r') as f:
        value_frame = None
        start_register = 0
        for line in f:
            if line.startswith('valueFrame'):
                value_frame = line.split('\t')[1].strip().strip('"')
            elif line.startswith('startRegister'):
                start_register = int(line.split('\t')[1].strip())
        
        if not value_frame:
            print(f"Error: No valueFrame found in {filename}")
            return
        print(f"Starting at register {start_register}")

    # Decode valueFrame
    data = decode_value_frame(value_frame)
    
    # Skip header (looks like 4 bytes of header + serial number)
    offset = 0
    while offset < len(data) and data[offset] != 0x00:
        offset += 1
    offset += 1  # Skip the null terminator
    
    # Parse and display register values
    print("\nRegister Values:")
    print("-" * 50)
    
    register_number = start_register  # Start at the specified register number
    while offset < len(data):
        try:
            if register_number in registers:
                reg_def = registers[register_number]
                value = parse_register_value(data, reg_def, offset)
                
                # Format the output
                if isinstance(value, dict):
                    print(f"Register {register_number} ({reg_def.get('description', 'Unknown')}):")
                    for k, v in value.items():
                        print(f"  {k}: {v}")
                else:
                    unit = reg_def.get('unit', '')
                    print(f"Register {register_number} ({reg_def.get('description', 'Unknown')}): {value} {unit}")
                
                # Update offset based on register type
                if 'num_values' in reg_def:
                    offset += 2  # Multi-value registers use 2 bytes
                elif reg_def.get('datatype') == 'float':
                    offset += 4  # Float values use 4 bytes
                else:
                    offset += 2  # Default to 2 bytes for other types
            else:
                # Print unknown registers with decimal, hex, and binary values
                value = struct.unpack('>H', data[offset:offset+2])[0]
                print(f"Register unknown-{register_number}: {value} (0x{value:04X}, 0b{value:016b})")
                offset += 2  # Unknown registers use 2 bytes
            
            register_number += 1
            
        except Exception as e:
            print(f"Error at offset {offset}: {e}")
            break

def main():
    # Set up command line argument parsing
    parser = argparse.ArgumentParser(description='Parse EG4 portal files and display register values')
    parser.add_argument('files', nargs='+', help='One or more portal files to process')
    args = parser.parse_args()

    # Load register definitions
    registers = load_register_definitions()

    # Process each file
    for filename in args.files:
        process_portal_file(filename, registers)

if __name__ == '__main__':
    main() 
