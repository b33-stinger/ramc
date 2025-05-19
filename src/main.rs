use std::fs::File;
use std::path::Path;
use std::usize;

use serde_json::{Value, json};
use clap::Parser;
use reqwest::{Error, Client, StatusCode};
use scraper::{Html, Selector, ElementRef};
use tokio::sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::io::{self, Write};
use std::time::Duration;

use once_cell::sync::Lazy;
use text_io::read;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, help = "URL to get the Mirrors from", default_value_t = String::from("https://archlinux.org/download/"))]
    download_url: String,

    #[arg(short, long, default_value_t = false, help = "Ask for a custom download URL (stdin)")]
    ask_custom_url: bool,

    #[arg(long, default_value_t = -1, help = "Max Mirrors to check (-1 == all)")]
    max_check: isize,

    #[arg(short, long, help = "Log file name", default_value_t =  String::from("log.json"))]
    log_file: String,

    #[arg(short, long, help = "Dump to log after each iteration", default_value_t = false)]
    continous_log: bool,

    #[arg(short, long, help = "Exclude Country", default_value_t = String::from(""))]
    exclude_country: String,

    #[arg(short, long, help = "Exclude Country", default_value_t = String::from(""))]
    include_country: String,

    #[arg(short, long, help = "Don't log (stdout) data", default_value_t = false)]
    quiet: bool,

    #[arg(short, long, help = "Don't log (file) data", default_value_t = false)]
    no_log: bool,

    #[arg(short, long, help = "Request Timeout", default_value_t = 30)]
    timeout: u64,

    #[arg(short, long, help = "User Agent", default_value_t = String::from(""))]
    user_agent: String,

    #[arg(short, long, help = "Disable SSL verification (For all)", default_value_t = false)]
    skip_ssl: bool,

    #[arg(short, long, help = "Maximum Threads to use (-1 == No Limit)", default_value_t = -1)]
    max_threads: i16,
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


fn parse_mirrors(raw_html: String, max_check: isize, country_exclude: Vec<&str>, country_include: Vec<&str>) -> Vec<Vec<String>> {
    let mut check_count = 0;

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
                let country = section.clone().to_lowercase();

                if !country_include.is_empty() && !country_include.contains(&&country.as_str()){
                    continue;
                }

                if !country_exclude.is_empty() && country_exclude.contains(&&country.as_str()){
                    continue;
                }

                if check_count == max_check{
                    return out;
                }
                check_count += 1;
                
                let url    = a.value().attr("href").unwrap_or("").to_string();
                let domain = a.text().collect::<Vec<_>>().join(" ").trim().to_string();
                out.push(vec![url, domain, country]);
            }
        }
    }
    out
}

#[tokio::main(flavor = "multi_thread")]
async fn check_mirror(quiet: bool) -> u8 {
    let args: Args = Args::parse();
    let max_threads: i16 = args.max_threads;
    let continous_log: bool = args.continous_log;
    let no_log = args.no_log;
    let mut tasks = vec![];
    let mirrors = MIRRORS.lock().unwrap();

    let client = Client::builder()
        .timeout(Duration::from_secs(args.timeout))
        .danger_accept_invalid_certs(args.skip_ssl)
        .user_agent(args.user_agent)
        .build()
        .expect("Failed to build client");

    let log_data = Arc::new(Mutex::new(LOG_DATA.lock().unwrap().clone()));

    let semaphore = if max_threads > 0 {
        Some(Arc::new(Semaphore::new(max_threads as usize)))
    } else {
        None
    };

    for mirror_data in mirrors.iter() {
        let log_data = Arc::clone(&log_data);
        let url = mirror_data[0].clone();
        let domain = mirror_data[1].clone();
        let country = mirror_data[2].clone();
        let client = client.clone();
        let semaphore = semaphore.clone();

        log_data.lock().unwrap().as_object_mut().unwrap().insert(
            url.clone(),
            json!({"domain": domain, "status": {}}),
        );

        tasks.push(tokio::spawn(async move {
            let _permit = match &semaphore {
                Some(sem) => Some(sem.acquire().await.unwrap()),
                None => None,
            };

            let response = client.get(&url).send().await;
            let mut done_mirrors = DONE_MIRRORS.lock().unwrap();
            let mut response_code = String::from("Error");
            let mut status_icon= "❌";

            match response {
                Ok(response) => {
                    if response.status() == StatusCode::OK {
                        log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["status"] = json!(1);
                        done_mirrors[0].0 += 1;
                        status_icon = "✅";
                    } else {
                        done_mirrors[0].1 += 1;
                        log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["status"] = json!(0);
                    }
                    response_code = response.status().as_u16().to_string();
                    log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["code"] = json!(response_code);
                    log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["country"] = json!(country);
                }
                Err(_) => {
                    log_data.lock().unwrap().as_object_mut().unwrap().get_mut(&url).unwrap()["status"] = json!(0);
                    done_mirrors[0].1 += 1;
                }
            }

            let percent_suceed: f32 = (done_mirrors[0].0 as f32 / (done_mirrors[0].0 + done_mirrors[0].1).max(1) as f32) * 100.0;
            let total_percent: f32 = ((done_mirrors[0].0 + done_mirrors[0].1) as f32 / done_mirrors[0].2 as f32) * 100.0;
            let mirrors_left = done_mirrors[0].2 - (done_mirrors[0].0 + done_mirrors[0].1);

            if !quiet {
                println!("{:<70} {:<30} {} ({})", url, country, status_icon, response_code);
                print!("[\x1b[31m{}\x1b[0m / \x1b[32m{}\x1b[0m ({:.2}%) / {} / ({}) {:.2}%]\r", done_mirrors[0].1, done_mirrors[0].0, percent_suceed, done_mirrors[0].2, mirrors_left, total_percent);
            }

            io::stdout().flush().unwrap();
            if continous_log && !no_log {
                log(Some(log_data.lock().unwrap().clone()));
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }

    if !no_log {
        log(Some(log_data.lock().unwrap().clone()));
    }

    1
}


#[tokio::main]
async fn get_mirrors(url: &str, max_mirrors: isize, country_exclude: Vec<&str>, country_include: Vec<&str>) -> Result<Vec<Vec<String>>, Error> {
    let response = reqwest::get(url).await?.text().await?;

    Ok(parse_mirrors(response, max_mirrors, country_exclude, country_include))
}

fn main() {
    // Reset the log file
    log(None);

    let mut args: Args = Args::parse();

    if args.ask_custom_url {
        print!("Download URL: [{}]\n# ", args.download_url);
        args.download_url = read!();
    }

    {
        let mut log_data = LOG_DATA.lock().unwrap();
        log_data["mirror_url"] = Value::String(String::from(&args.download_url));
        log(Some(log_data.clone()));
    }
    {
        let country_exclude: Vec<&str> = args.exclude_country.split(',').filter(|s| !s.is_empty()).collect();
        let country_include: Vec<&str> = args.include_country.split(',').filter(|s| !s.is_empty()).collect();
        let mut mirrors = MIRRORS.lock().unwrap();
        *mirrors = get_mirrors(&args.download_url, args.max_check, country_exclude, country_include).unwrap();
    }
    {
        let mirrors = MIRRORS.lock().unwrap();
        DONE_MIRRORS.lock().unwrap()[0].2 = mirrors.len();
    }
    check_mirror(args.quiet);
}
