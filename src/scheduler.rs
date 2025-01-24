use std::time::{Duration, SystemTime};
use std::{fs::OpenOptions, io::Write, thread};

use crate::data::{self, Credentials, Error};

#[derive(PartialEq, Default)]
pub enum RepeatMode {
    #[default]
    Once,
    Repeat(u64),
}

pub struct Scheduler {
    mode: RepeatMode,
    max_retries: u8,
    file_path: String,
    creds: Credentials,
    last_update: SystemTime,
}

impl Scheduler {
    pub fn new(mode: RepeatMode, max_retries: u8, file_path: String, creds: Credentials) -> Self {
        Self {
            mode,
            max_retries,
            file_path,
            creds,
            last_update: SystemTime::now(),
        }
    }

    fn load_data(&self) -> Result<Vec<(String, String)>, Error> {
        let mut attempts = 0;
        loop {
            match data::get_data(&data::log_in(&self.creds)?, self.max_retries) {
                Ok(data) => return Ok(data),
                Err(Error::RequestFailed(error)) => {
                    if attempts >= 3 {
                        panic!("Retrieving data failed: {}", error);
                    }
                    attempts += 1;
                }
                Err(error) => return Err(error),
            };
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        if self.mode == RepeatMode::Once {
            let data = self.load_data()?;
            self.save_data(data);
        } else if let RepeatMode::Repeat(repeat) = self.mode {
            tracing::info!("Scheduler started, repeat every {} seconds", repeat);

            loop {
                if SystemTime::now().duration_since(self.last_update).unwrap()
                    >= Duration::from_secs(repeat)
                {
                    self.last_update = SystemTime::now();

                    if let Ok(data) = self.load_data() {
                        self.save_data(data);
                    }
                }

                thread::sleep(Duration::from_secs(1));
            }
        }
        Ok(())
    }

    fn save_data(&self, data: Vec<(String, String)>) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .expect("Failed to open a file");

        let mut values = String::new();

        values.push_str(
            &chrono::Local::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
                .to_string(),
        );

        if data.is_empty() {
            values.push_str("n/a");
        }

        for (_, value) in data.iter() {
            values.push(';');
            values.push_str(value);
        }

        values.push('\n');

        file.write_all(values.as_bytes())
            .expect("Failed writing to file.");
    }
}
