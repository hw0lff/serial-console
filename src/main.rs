use log::trace;
use std::io::{stdin, stdout, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use structopt::StructOpt;
use termion::raw::IntoRawMode;
use termion::screen::*;

#[derive(Debug, StructOpt)]
#[structopt(author)]
struct SC {
    /// Baud rate to connect at
    #[structopt(name = "baud rate", short, long = "baudrate", default_value = "9600")]
    baud_rate: u32,

    /// Device path to a serial port
    #[structopt(parse(from_str))]
    device: String,
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

fn main() {
    let sc_opt: SC = SC::from_args();

    let mut stdin = stdin();
    let screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    let screen = Arc::new(Mutex::new(screen));

    write_start_screen_msg(&mut screen.clone());

    // let ports = serialport::available_ports().expect("No ports found!");
    // for p in ports {
    //     write!(screen.lock().unwrap(), "{}\n\r", p.port_name).unwrap();
    // }
    // screen.lock().unwrap().flush().unwrap();

    let port = serialport::new(sc_opt.device.clone(), sc_opt.baud_rate)
        .timeout(Duration::from_millis(1))
        .open();

    let port = match port {
        Ok(sp) => sp,
        Err(err) if err.kind() == serialport::ErrorKind::Io(std::io::ErrorKind::NotFound) => {
            eprint!("{}Port not Found: {}\n\r", ToMainScreen, sc_opt.device);
            screen.lock().unwrap().suspend_raw_mode().unwrap();
            std::process::exit(1);
        }
        Err(err) => {
            eprint!(
                "{}Error opening port, please report this: {:?}\n\r",
                ToMainScreen, err
            );
            screen.lock().unwrap().suspend_raw_mode().unwrap();
            std::process::exit(1);
        }
    };

    let mut serial_port_in = port.try_clone().unwrap();
    let mut serial_port_out = port.try_clone().unwrap();

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
