use data::{Credentials, Error};
use scheduler::{RepeatMode, Scheduler};
use std::env;

mod data;
mod scheduler;

fn main() -> Result<(), Error> {
    let mut repeat = RepeatMode::Once;
    let mut file_path = "data.csv".to_owned();
    let mut data_retries = 5;

    let mut csl = "".to_owned();
    let mut hsl = "".to_owned();
    let mut year = "2025".to_owned();

    let mut args = env::args();
    args.next();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" => {
                println!("Options:");
                println!(
                    "--repeat <seconds> - how frequently should the sensor data be checked, if ommited only check once"
                );
                println!(
                    "--retries <number> - how many times will the sensor data reading be attempted before returning"
                );
                println!("--info - print debug info");
                println!("--file <file_path> - specify where the log should be saved");
                println!("--csl <number> - machine number");
                println!("--hsl <number> - machine password");
                println!("--year <year> - year to check data for");

                return Ok(());
            }
            "--repeat" => {
                repeat = RepeatMode::Repeat(
                    args.next()
                        .expect("--repeat <seconds>")
                        .parse()
                        .expect("Repeat has to be specified as a valid uint64"),
                );
            }
            "--retries" => {
                data_retries = args
                    .next()
                    .expect("--retries <num>")
                    .parse::<u8>()
                    .expect("Retries has to be a valid uint8")
            }
            "--info" => {
                tracing_subscriber::fmt().init(); // Setup logging
            }
            "--file" => {
                file_path = args.next().expect("--file <file_path>");
            }
            "--csl" => csl = args.next().expect("--csl <number>"),
            "--hsl" => hsl = args.next().expect("--hsl <password>"),
            "--year" => year = args.next().expect("--year <year>"),
            _ => panic!("Argument '{}' not supported", arg),
        }
    }

    Scheduler::new(
        repeat,
        data_retries,
        file_path,
        Credentials::new(csl, hsl, year),
    )
    .start()?;

    Ok(())
}
