use std::fs::File;
use std::path::Path;
use std::usize;

use serde_json::{Value, json};
use clap::Parser;
use reqwest::{Error, Client, StatusCode};
use scraper::{Html, Selector, ElementRef};
use std::sync::{Arc, Mutex};
use std::io::{self, Write};
use std::time::Duration;

use once_cell::sync::Lazy;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, help = "URL to get the Mirrors from", default_value_t = String::from("https://archlinux.org/download/"))]
    download_url: String,

    //#[arg(short, long, default_value_t = false)]
    //ask_custom_url: bool,

    //#[arg(short, long, default_value_t = 0)]
    //max_check: usize,

    #[arg(short, long, help = "log file name", default_value_t =  String::from("log.json"))]
    log_file: String,

    #[arg(short, long, help = "dump to log after each iteration", default_value_t = false)]
    continous_log: bool,
}
/*
DONE_MIRRORS[0].0 = Working     (amount)
DONE_MIRRORS[0].1 = Not Working (amount)
DONE_MIRRORS[0].2 = All Mirros  (len)
*/
static DONE_MIRRORS: Lazy<Mutex<Vec<(usize, usize, usize)>>> = Lazy::new(|| { Mutex::new(vec![(0, 0, 0)]) });

/*
MIRRORS[x].0 = URL
MIRRORS[x].1 = Domain
MIRRORS[x].2 = Country
*/
static MIRRORS: Lazy<Mutex<Vec<Vec<String>>>> = Lazy::new(|| { Mutex::new(Vec::new()) });
/*
LOG_DATA["mirror_url"]          = Mirror URL
LOG_DATA["some_url"]["code"]    = HTTP response Code
LOG_DATA["some_url"]["domain"]  = URL domain
LOG_DATA["some_url"]["status"]  = if 1 == Working else Not Working
*/
static LOG_DATA: Lazy<Mutex<serde_json::Value>> = Lazy::new(|| { Mutex::new(json!({}))});


fn log(data: Option<serde_json::Value>) -> u8 {
    let data = data.unwrap_or(json!({}));
    let args: Args = Args::parse();
    let path = Path::new(args.log_file.as_str());
    let display = path.display();

    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    match file.write_all(serde_json::to_string_pretty(&data).unwrap().as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => {}
    }
    1
}


fn parse_mirrors(raw_html: String) -> Vec<Vec<String>> {
    let document = Html::parse_document(&raw_html);
    
    let div_sel    = Selector::parse("div#download-mirrors").unwrap();
    let container  = document.select(&div_sel).next().expect("no container div");

    let both_sel   = Selector::parse("h5, ul").unwrap();
    let link_sel   = Selector::parse("a").unwrap();

    let elems: Vec<ElementRef> = container.select(&both_sel).collect();

    let mut out = Vec::new();
    for chunk in elems.chunks(2) {
        if let [h5, ul] = chunk {
            let section = h5
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            for a in ul.select(&link_sel) {
                let url    = a.value().attr("href").unwrap_or("").to_string();
                let domain = a.text().collect::<Vec<_>>().join(" ").trim().to_string();
                out.push(vec![url, domain, section.clone()]);
            }
        }
    }

    out
}

#[tokio::main(flavor = "multi_thread")]
async fn check_mirror() -> u8 {
    let args: Args = Args::parse();
    let continous_log: bool = args.continous_log;
    let mut tasks = vec![];
    let mirrors = MIRRORS.lock().unwrap();

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build client");

    let log_data = Arc::new(Mutex::new(LOG_DATA.lock().unwrap().clone()));

    for mirror_data in mirrors.iter() {
        let log_data = Arc::clone(&log_data);
        let url = mirror_data[0].clone();
        let domain = mirror_data[1].clone();
        //let country = mirror_data[2].clone();
        let client = client.clone();

        log_data.lock().unwrap().as_object_mut().unwrap().insert(
            url.clone(),
            json!({"domain": domain, "status": {}}),
        );


        tasks.push(tokio::spawn(async move {
            let response = client.get(&url).send().await;
            let mut done_mirrors = DONE_MIRRORS.lock().unwrap();
            match response {
                Ok(response) => {
                    if response.status() == StatusCode::OK {
                        log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["status"] = json!(1);
                        done_mirrors[0].0 += 1;
                    } else {
                        log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["status"] = json!(0);
                    }
                    log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["code"] = json!(response.status().as_u16());
                }
                Err(_) => {
                    log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["status"] = json!(0);
                    done_mirrors[0].1 += 1;
                }
            }
            print!("\r({} / {} / {})", done_mirrors[0].0, done_mirrors[0].1, done_mirrors[0].2);
            io::stdout().flush().unwrap();
            if continous_log{
                log(Some(log_data.lock().unwrap().clone()));
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }
    log(Some(log_data.lock().unwrap().clone()));
    1
}


#[tokio::main]
async fn get_mirrors(url: &str) -> Result<Vec<Vec<String>>, Error> {
    let response = reqwest::get(url).await?.text().await?;

    Ok(parse_mirrors(response))
}

fn main() {
    // Reset the log file
    log(None);

    let args: Args = Args::parse();
    {
        let mut log_data = LOG_DATA.lock().unwrap();
        log_data["mirror_url"] = Value::String(String::from(&args.download_url));
        log(Some(log_data.clone()));
    }
    {
        let mut mirrors = MIRRORS.lock().unwrap();
        *mirrors = get_mirrors(&args.download_url).unwrap();
    }
    {
        let mirrors = MIRRORS.lock().unwrap();
        DONE_MIRRORS.lock().unwrap()[0].2 = mirrors.len();
    }
    check_mirror();
}
