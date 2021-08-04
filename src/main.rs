use structopt::StructOpt;
use std::path::PathBuf;

#[derive(Debug,StructOpt)]
struct SC {
    /// Baud rate to connect at
    #[structopt(short,long)]
    baudrate: Option<u32>,

    /// Device path to a serial port
    #[structopt(parse(from_os_str))]
    device: PathBuf,
}

fn main() {
    let sc = SC::from_args();
    println!("{:?}", sc);
}
