use reqwest::{blocking::multipart, header::HeaderMap};
use std::{thread, time::Duration};

pub struct Credentials {
    csl: String,
    hsl: String,
    year: String,
}

impl Credentials {
    pub fn new(csl: String, hsl: String, year: String) -> Self {
        Self { csl, hsl, year }
    }
}

#[derive(Debug)]
pub enum Error {
    MaxRetriesReached,
    RequestFailed(String),
}

// Retrieves the session ID
pub fn log_in(creds: &Credentials) -> Result<String, Error> {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("System error: failed to build client");

    let form = [
        ("csl", &creds.csl),
        ("hsl", &creds.hsl),
        ("rok", &creds.year),
    ];

    let req = client
        .post("http://vpktc.eu/index.php")
        .form(&form)
        .build()
        .expect("System error: failed to build request");

    let Ok(resp) = client.execute(req) else {
        return Err(Error::RequestFailed("Login request unsuccessful".into()));
    };

    if resp.status() != 302 {
        return Err(Error::RequestFailed(format!(
            "Log in failed: {}",
            resp.status()
        )));
    }

    let hv = resp
        .headers()
        .get("Set-Cookie")
        .expect("Set-Cookie header not present");

    let cookies = hv.to_str().unwrap().to_owned();
    let session_id = cookies.split(";").next().unwrap();

    Ok(session_id.to_owned())
}

fn raw_data(login_cookie: &str, max_retries: u8, code: &str) -> Result<String, Error> {
    let mut headers = HeaderMap::new();
    headers.insert("Cookie", login_cookie.parse().unwrap());

    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .expect("System error: failed to build client");

    let mut retries = 0;

    let data = loop {
        let form = multipart::Form::new().text("kod", code.to_owned());

        let req = client
            .post("http://vpktc.eu/obec.php")
            .multipart(form)
            .build()
            .expect("System error: failed to build request");

        let Ok(resp) = client.execute(req) else {
            return Err(Error::RequestFailed("Request not sent successfully".into()));
        };
        if resp.status() != 200 {
            return Err(Error::RequestFailed(format!(
                "Server error: {}",
                resp.status()
            )));
        }
        let Ok(data) = resp.text() else {
            return Err(Error::RequestFailed("No response body".into()));
        };

        if data.is_empty() {
            if retries >= max_retries {
                return Err(Error::MaxRetriesReached);
            }

            thread::sleep(Duration::from_secs(3));
            retries += 1;
            continue;
        }
        break data;
    };
    tracing::info!(
        "Sensor data: {} - retries: {}/{}",
        code,
        retries,
        max_retries
    );

    Ok(data)
}

// Gets the sensor data
pub fn get_data(login_cookie: &str, max_retries: u8) -> Result<Vec<(String, String)>, Error> {
    let data_m = raw_data(login_cookie, max_retries, "M")?;
    let data_k = raw_data(login_cookie, max_retries, "K")?;

    let mut parsed_data = Vec::new();

    for pair in data_m.split("\n") {
        if let Some((index, value)) = pair.split_once(";") {
            tracing::info!("{}. {}", index, value);

            parsed_data.push((index.to_string(), value.to_string()));
        } else if !pair.is_empty() {
            tracing::error!("Malformed data: {}", pair);

            parsed_data.push(("n/a".to_owned(), "malformed".to_owned()))
        }
    }

    for pair in data_k.split("\n") {
        if let Some((index, value)) = pair.split_once(";") {
            if index == "36" || index == "38" {
                tracing::info!("{}. {}", index, value);

                parsed_data.push((index.to_string(), value.to_string()));
            }
        }
    }

    Ok(parsed_data)
}
