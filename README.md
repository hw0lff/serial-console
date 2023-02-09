# Scip - Serial Console Interfacing Program
A command line tool to communicate with a serial console written in [Rust](https://rust-lang.org)

## Installation
```bash
cargo install serial-console
```

## Usage
```
USAGE:
    scip <DEVICE> [ARGS]

ARGS:
    <DEVICE>          Set the device path to a serial port
    <baud rate>       Set the baud rate to connect at [default: 9600]
    <data bits>       Set the number of bits used per character [default: 8] [possible values:
                      5, 6, 7, 8]
    <parity>          Set the parity checking mode [default: N] [possible values: N, O, E]
    <stop bits>       Set the number of stop bits transmitted after every character [default: 1]
                      [possible values: 1, 2]
    <flow control>    Set the flow control mode [default: N] [possible values: N, H, S]

Escape commands begin with <Enter> and end with one of the following sequences:
    ~~ - send the '~' character
    ~. - terminate the connection
```

For more verbose help information and parameter suggestions add the `--help` option:
```bash
scip --help
```

## Examples
```bash
scip /dev/ttyUSB0 115200
scip /dev/ttyUSB1 19200 6 E 2 H
```

## License
MIT
