use std::io::{self, stdin, stdout, Read, Write};
use std::sync::mpsc::{channel, TryRecvError};
use std::thread;
use std::time::Duration;

use clap::Parser;
use serialport::{DataBits, FlowControl, Parity, SerialPortBuilder, StopBits};
use termion::raw::IntoRawMode;
use termion::screen::*;

#[derive(Debug, Parser)]
#[clap(
    author,
    after_help = "\
Escape commands begin with <Enter> and end with one of the following sequences:
    ~~ - sends the '~' character
    ~. - terminates the connection
",
    version
)]
struct SC {
    /// Sets the device path to a serial port
    #[clap(parse(from_str))]
    device: String,

    /// Sets the baud rate to connect at
    #[clap(
        name = "baud rate",
        default_value = "9600",
        long_help = r"Sets the baud rate to connect at

Common values: 300, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 500000, 576000, 921600, 1000000, 1152000, 1500000, 2000000, 2500000, 3000000, 3500000, 4000000
"
    )]
    baud_rate: u32,

    /// Sets the number of bits used per character
    #[clap(
        name = "data bits",
        default_value = "8",
        possible_values = &["5", "6", "7", "8"],
    )]
    data_bits: u8,
    /// Sets the parity checking mode
    #[clap(
        name = "parity",
        default_value = "N",
        ignore_case = true,
        possible_values = &["N","O","E"],
        long_help = r"Sets the parity checking mode

Possible values:
    - N, n => None
    - O, o => Odd
    - E, e => Even
"
    )]
    parity: String,
    /// Sets the number of stop bits transmitted after every character
    #[clap(
        name = "stop bits",
        default_value = "1",
        possible_values = &["1", "2"],
    )]
    stop_bits: u8,
    /// Sets the flow control mode
    #[clap(
        name = "flow control",
        default_value = "N",
        ignore_case = true,
        possible_values = &["N","H","S"],
        long_help = r"Sets the flow control mode

Possible values:
    - N, n => None
    - H, h => Hardware    # uses XON/XOFF bytes
    - S, s => Software    # uses RTS/CTS signals
"
    )]
    flow_control: String,
}

enum EscapeState {
    // Wait for Enter
    WaitForEnter,
    // Wait for escape character
    WaitForEC,
    // Ready to process command
    ProcessCMD,
}

fn main() {
    let sc_args: SC = SC::parse();

    let port_builder: SerialPortBuilder = parse_arguments_into_serialport(&sc_args);
    let port;

    match port_builder.open() {
        Ok(sp) => port = sp,
        Err(err) if err.kind() == serialport::ErrorKind::Io(io::ErrorKind::NotFound) => {
            eprint!("Device not found: {}\n\r", sc_args.device);
            return;
        }
        Err(err) => {
            eprint!("Error opening port, please report this: {:?}\n\r", err);
            return;
        }
    };
    let mut serial_port_in = port.try_clone().unwrap();
    let mut serial_port_out = port.try_clone().unwrap();

    let mut stdin = stdin();
    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());

    write_start_screen_msg(&mut screen);

    let (tx, rx) = channel::<([u8; 512], usize)>();

    let _terminal_stdin = thread::spawn(move || loop {
        let mut data = [0; 512];
        let n = stdin.read(&mut data[..]).unwrap();
        tx.send((data, n)).unwrap();
    });

    let mut escape_state: EscapeState = EscapeState::WaitForEnter;
    loop {
        let mut serial_bytes = [0; 512];
        match serial_port_out.read(&mut serial_bytes[..]) {
            Ok(n) => {
                if n > 0 {
                    screen.write_all(&serial_bytes[..n]).unwrap();
                    screen.flush().unwrap();
                }
            }
            Err(err) if err.kind() == io::ErrorKind::TimedOut => {}
            Err(err) if err.kind() == io::ErrorKind::BrokenPipe => {
                eprint!("{}Device disconnected\n\r", ToMainScreen);
                break;
            }
            Err(err) => {
                eprint!("{}{}\n\r", ToMainScreen, err);
                break;
            }
        }

        let data: [u8; 512];
        let n: usize;
        match rx.try_recv() {
            Ok((rx_data, rx_n)) => {
                data = rx_data;
                n = rx_n;
            }
            Err(TryRecvError::Disconnected) => {
                eprint!("{}Error: Stdin reading thread stopped.\n\r", ToMainScreen,);
                break;
            }
            Err(TryRecvError::Empty) => {
                continue;
            }
        }

        if n == 1 {
            match escape_state {
                EscapeState::WaitForEnter => {
                    if data[0] == b'\r' || data[0] == b'\n' {
                        escape_state = EscapeState::WaitForEC;
                    }
                }
                EscapeState::WaitForEC => match data[0] {
                    b'~' => {
                        escape_state = EscapeState::ProcessCMD;
                        continue;
                    }
                    b'\r' => {
                        escape_state = EscapeState::WaitForEC;
                    }
                    _ => {
                        escape_state = EscapeState::WaitForEnter;
                    }
                },
                EscapeState::ProcessCMD => match data[0] {
                    b'.' => {
                        break;
                    }
                    b'\r' => {
                        escape_state = EscapeState::WaitForEC;
                    }
                    _ => {
                        escape_state = EscapeState::WaitForEnter;
                    }
                },
            }
        }

        // try to write terminal input to serial port
        match serial_port_in.write(&data[..n]) {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::TimedOut => {}
            Err(err) => {
                eprint!("{}{}\n\r", ToMainScreen, err);
                break;
            }
        }
    }
}

fn parse_arguments_into_serialport(sc_args: &SC) -> SerialPortBuilder {
    fn match_data_bits(data_bits: u8) -> DataBits {
        match data_bits {
            8 => DataBits::Eight,
            7 => DataBits::Seven,
            6 => DataBits::Six,
            5 => DataBits::Five,
            _ => DataBits::Eight,
        }
    }
    fn match_parity(parity: &str) -> Parity {
        match parity {
            "N" | "n" => Parity::None,
            "O" | "o" => Parity::Odd,
            "E" | "e" => Parity::None,
            _ => Parity::None,
        }
    }
    fn match_stop_bits(stop_bits: u8) -> StopBits {
        match stop_bits {
            1 => StopBits::One,
            2 => StopBits::Two,
            _ => StopBits::One,
        }
    }
    fn match_flow_control(flow_control: &str) -> FlowControl {
        match flow_control {
            "N" | "n" => FlowControl::None,
            "H" | "h" => FlowControl::Hardware,
            "S" | "s" => FlowControl::Software,
            _ => FlowControl::None,
        }
    }
    let path: &str = &sc_args.device;
    let baud_rate: u32 = sc_args.baud_rate;
    let data_bits: DataBits = match_data_bits(sc_args.data_bits);
    let parity: Parity = match_parity(sc_args.parity.as_str());
    let stop_bits: StopBits = match_stop_bits(sc_args.stop_bits);
    let flow_control: FlowControl = match_flow_control(sc_args.flow_control.as_str());
    let timeout: Duration = Duration::from_millis(10);

    serialport::new(path, baud_rate)
        .data_bits(data_bits)
        .parity(parity)
        .stop_bits(stop_bits)
        .flow_control(flow_control)
        .timeout(timeout)
}

fn write_start_screen_msg(screen: &mut impl Write) {
    write!(
        screen,
        "{}{}Welcome to serial console.{}To exit type <Enter> + ~ + .\r\nor unplug the serial port.{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1),
        termion::cursor::Goto(1, 2),
        termion::cursor::Goto(1, 4)
    )
    .unwrap();
    screen.flush().unwrap();
}
