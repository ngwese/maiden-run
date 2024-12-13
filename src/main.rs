use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::Path;

use clap::{Arg, Command};
use clap::parser::ValuesRef;

use http::Request;
use tungstenite::{connect, Message};
use url::Url;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new(env!("CARGO_CRATE_NAME")) //App::new("maiden-run")
        .about("run a script immediately or when files change")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("endpoint")
                .help("matron socket")
                .short('e')
                .long("endpoint")
                .value_name("URL")
                .default_value("ws://localhost:5555/")
                // .takes_value(true),
        )
        .arg(
            Arg::new("watch")
                .help("auto run on script/dir changes")
                .short('w')
                .long("watch")
                // .takes_value(false),
        )
        .arg(
            Arg::new("script")
                .help("script file to play")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("dirs")
                .help("directories to watch")
                .index(2)
                .num_args(1..),
        )
        .get_matches();

    // collect up arguments
    let endpoint = matches.get_one::<&str>("endpoint").unwrap();
    println!("endpoint: {}", endpoint);

    let watch = matches.contains_id("watch");
    println!("watch: {}", watch);

    let script = matches.get_one::<&str>("script").unwrap();
    println!("script: {}", script);

    let dirs = matches
        .get_many("dirs")
        .unwrap_or(ValuesRef::default())
        .collect();
    println!("dirs: {:?}", dirs);

    // parse the url for validity
    let endpoint_url = match Url::parse(endpoint) {
        Ok(url) => url,
        Err(e) => panic!("Invalid endpoint url syntax: {}", e),
    };

    if endpoint_url.scheme() != "ws" {
        panic!("Endpoint url must have ws:// scheme")
    }

    // run script or setup watch
    if watch {
        do_watch(endpoint, script, &dirs)?;
    } else {
        do_run(endpoint, script);
    }

    Ok(())
}

fn do_run<P: AsRef<Path>>(endpoint: &str, script: P) {
    // build http request directly so we can specify the websocket protocol
    let request = Request::builder()
        .uri(endpoint)
        .header("Sec-WebSocket-Protocol", "bus.sp.nanomsg.org")
        .body(())
        .unwrap();
    let connection = match connect(request) {
        Ok(conn) => Some(conn),
        Err(err) => {
            println!("Connection failed: {}", err);
            None
        }
    };

    if let Some((mut s, _)) = connection {
        let code = format!("norns.script.load(\"{}\")\n\0", script.as_ref().display());
        println!("Sending: {}", code);
        match s.send(Message::Text(code)) {
            Ok(_) => {}
            Err(e) => {
                println!("Writing to socket failed: {}", e)
            }
        }
    };
}

fn do_watch<P: AsRef<Path>>(endpoint: &str, script: P, dirs: &Vec<P>) -> notify::Result<()> {
    let (tx, rx) = channel();
    let config = Config::default().with_poll_interval(Duration::from_secs(1));
    let mut watcher = RecommendedWatcher::new(tx, config)?;

    watcher.watch(script.as_ref(), RecursiveMode::NonRecursive)?;
    for dir in dirs {
        watcher.watch(dir.as_ref(), RecursiveMode::Recursive).unwrap(); // FIXME: error handling
    }

    for response in rx {
        match response {
            Ok(event) => match event.kind {
              EventKind::Modify(_) => {
                    println!("{:?}", event);
                    do_run(endpoint, &script)
                }
                _ => {}
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
