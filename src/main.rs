use log::trace;
use serialport::SerialPortBuilder;
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::io::{stdin, stdout, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use structopt::StructOpt;
use termion::raw::IntoRawMode;
use termion::screen::*;

#[derive(Debug, StructOpt)]
#[structopt(author, global_setting(structopt::clap::AppSettings::ColoredHelp))]
struct SC {
    /// Sets the device path to a serial port
    #[structopt(parse(from_str))]
    device: String,

    /// Sets the baud rate to connect at
    #[structopt(
        name = "baud rate",
        default_value = "9600",
        long_help = r"Sets the baud rate to connect at

Common values: 300, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 500000, 576000, 921600, 1000000, 1152000, 1500000, 2000000, 2500000, 3000000, 3500000, 4000000
"
    )]
    baud_rate: u32,

    /// Sets the number of bits used per character
    #[structopt(
        name = "data bits",
        default_value = "8",
        long_help = r"Sets the number of bits per character

Possible values:
    - 5
    - 6
    - 7
    - 8
"
    )]
    data_bits: u8,
    /// Sets the parity checking mode
    #[structopt(
        default_value = "None",
        long_help = r"Sets the parity checking mode

Possible values:
    - None, N, n => None
    - Odd,  O, o => Odd
    - Even, E, e => Even
"
    )]
    parity: String,
    /// Sets the number of stop bits transmitted after every character
    #[structopt(
        name = "stop bits",
        default_value = "1",
        long_help = r"Sets the number of stop bits transmitted after every character

Possible values:
    - 1
    - 2
"
    )]
    stop_bits: u8,
    /// Sets the flow control mode
    #[structopt(
        name = "flow control",
        default_value = "None",
        long_help = r"Sets the flow control mode

Possible values:
    - None, N, n => None
    - Hardware, Hard, H, h => Hardware    # uses XON/XOFF bytes
    - Software, Soft, S, s => Software    # uses RTS/CTS signals
"
    )]
    flow_control: String,
}

fn main() {
    let sc_args: SC = SC::from_args();

    let port: SerialPortBuilder = parse_arguments_into_serialport(&sc_args);
    let port = match port.open() {
        Ok(sp) => sp,
        Err(err) if err.kind() == serialport::ErrorKind::Io(std::io::ErrorKind::NotFound) => {
            eprint!("Device not found: {}\n\r", sc_args.device);
            std::process::exit(1);
        }
        Err(err) => {
            eprint!("Error opening port, please report this: {:?}\n\r", err);
            std::process::exit(1);
        }
    };
    let mut serial_port_in = port.try_clone().unwrap();
    let mut serial_port_out = port.try_clone().unwrap();

    let mut stdin = stdin();
    let screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    let screen = Arc::new(Mutex::new(screen));

    write_start_screen_msg(&mut screen.clone());

    let screen_from = screen.clone();
    let _from_serial_handle = thread::spawn(move || loop {
        let mut serial_bytes = [0; 512];
        match serial_port_out.read(&mut serial_bytes[..]) {
            Ok(n) => {
                if n > 0 {
                    screen_from
                        .lock()
                        .unwrap()
                        .write_all(&serial_bytes[..n])
                        .unwrap();
                    screen_from.lock().unwrap().flush().unwrap();
                }
            }
            Err(ref err) if err.kind() == std::io::ErrorKind::TimedOut => {}
            // todo: catch broken pipe error
            Err(err) => {
                eprint!("{}{}\n\r", ToMainScreen, err);
                break;
            }
        }
    });

    let _to_serial_handle: JoinHandle<_> = thread::spawn(move || loop {
        let mut data = [0; 512];
        let n = stdin.read(&mut data).unwrap();

        match serial_port_in.write(&data[..n]) {
            Ok(n) => {
                trace!("wrote {} bytes", n);
            }
            Err(ref err) if err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => {
                eprint!("{}{}\n\r", ToMainScreen, err);
                break;
            }
        }
    });

    _from_serial_handle.join().unwrap();
    // _to_serial_handle.join();
    screen.lock().unwrap().flush().unwrap();
}

fn parse_arguments_into_serialport(sc_args: &SC) -> SerialPortBuilder {
    let path: String = sc_args.device.clone();
    let baud_rate: u32 = sc_args.baud_rate;
    let data_bits: DataBits = match sc_args.data_bits {
        8 => DataBits::Eight,
        7 => DataBits::Seven,
        6 => DataBits::Six,
        5 => DataBits::Five,
        _ => DataBits::Eight,
    };
    let parity: Parity = match sc_args.parity.as_str() {
        "None" | "N" | "n" => Parity::None,
        "Odd" | "O" | "o" => Parity::Odd,
        "Even" | "E" | "e" => Parity::None,
        _ => Parity::None,
    };
    let stop_bits: StopBits = match sc_args.stop_bits {
        1 => StopBits::One,
        2 => StopBits::Two,
        _ => StopBits::One,
    };
    let flow_control: FlowControl = match sc_args.flow_control.as_str() {
        "None" | "N" | "n" => FlowControl::None,
        "Hardware" | "Hard" | "H" | "h" => FlowControl::Hardware,
        "Software" | "Soft" | "S" | "s" => FlowControl::Software,
        _ => FlowControl::None,
    };
    let timeout: Duration = Duration::from_millis(10);

    serialport::new(path, baud_rate)
        .data_bits(data_bits)
        .parity(parity)
        .stop_bits(stop_bits)
        .flow_control(flow_control)
        .timeout(timeout)
}

fn write_start_screen_msg<W: Write>(screen: &mut Arc<Mutex<W>>) {
    let screen = screen.clone();
    write!(
        screen.lock().unwrap(),
        "{}{}Welcome to serial console.{}To exit unplug the serial port.{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1),
        termion::cursor::Goto(1, 2),
        termion::cursor::Goto(1, 4)
    )
        .unwrap();
    screen.lock().unwrap().flush().unwrap();
}
