use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::Path;
use std::env;

use clap::{Arg, Command};
use clap::parser::ValuesRef;

use tungstenite::{connect, Message, client::IntoClientRequest};
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
                .default_value("ws://norns.local:5555/")
                // .takes_value(true),
        )
        .arg(
            Arg::new("no-watch")
                .help("disable auto-run on file changes (watch is on by default)")
                .long("no-watch")
                .action(clap::ArgAction::SetTrue)
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
    let endpoint = matches.get_one::<String>("endpoint").unwrap();
    println!("endpoint: {}", endpoint);

    let watch = !matches.get_flag("no-watch");
    println!("watch: {}", watch);

    let script = matches.get_one::<String>("script").unwrap();
    println!("script: {}", script);

    let default_dir = String::from(".");
    let mut dirs: Vec<&String> = matches
        .get_many("dirs")
        .unwrap_or(ValuesRef::default())
        .collect();

    // If watch is enabled and no dirs specified, default to "."
    if watch && dirs.is_empty() {
        dirs = vec![&default_dir];
    }

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

/// Extract the origin header value from an endpoint URL
fn extract_origin(endpoint: &str) -> String {
    if let Ok(url) = url::Url::parse(endpoint) {
        format!("{}://{}", url.scheme(), url.host_str().unwrap_or("localhost"))
    } else {
        "http://localhost".to_string()
    }
}

/// Convert a local file path to a norns script path
/// If it's a simple name (no path separators), assumes it's in code/name/name.lua
/// If it's a path, passes it through as-is
fn path_to_norns_path(path: &Path) -> String {
    let path_str = path.to_str().unwrap();

    // Check if this is a simple name (no path separators)
    if !path_str.contains('/') && !path_str.contains('\\') {
        // Simple name case: foo or foo.lua
        let name = if let Some(stripped) = path_str.strip_suffix(".lua") {
            stripped
        } else {
            path_str
        };
        format!("code/{}/{}.lua", name, name)
    } else {
        // It's a path, return as-is
        path_str.to_string()
    }
}

fn do_run<P: AsRef<Path>>(endpoint: &str, script: P) {
    // build http request directly so we can specify the websocket protocol
    let mut request = endpoint.into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        "bus.sp.nanomsg.org".parse().unwrap()
    );

    // Extract the host from the endpoint for the Origin header
    let origin = extract_origin(endpoint);

    request.headers_mut().insert(
        "Origin",
        origin.parse().unwrap()
    );

    println!("Attempting to connect to: {}", endpoint);
    let connection = match connect(request) {
        Ok(conn) => {
            println!("Connection successful!");
            Some(conn)
        },
        Err(err) => {
            println!("Connection failed: {}", err);
            println!("Error details: {:?}", err);
            None
        }
    };

    if let Some((mut s, _)) = connection {
        // Convert script path to norns path
        let script_path = script.as_ref();
        let norns_path = path_to_norns_path(script_path);

        let code = format!("norns.script.load(\"{}\")\n\0", norns_path);
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

    // Only watch the script file if it's an actual path (contains path separators)
    let script_str = script.as_ref().to_str().unwrap();
    if script_str.contains('/') || script_str.contains('\\') {
        watcher.watch(script.as_ref(), RecursiveMode::NonRecursive)?;
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_origin_localhost() {
        assert_eq!(extract_origin("ws://localhost:5555/"), "ws://localhost");
    }

    #[test]
    fn test_extract_origin_norns_local() {
        assert_eq!(extract_origin("ws://norns.local:5555/"), "ws://norns.local");
    }

    #[test]
    fn test_extract_origin_custom_host() {
        assert_eq!(extract_origin("ws://192.168.1.100:5555/"), "ws://192.168.1.100");
    }

    #[test]
    fn test_extract_origin_https() {
        assert_eq!(extract_origin("wss://norns.local:5555/"), "wss://norns.local");
    }

    #[test]
    fn test_extract_origin_with_path() {
        assert_eq!(extract_origin("ws://norns.local:5555/some/path"), "ws://norns.local");
    }

    #[test]
    fn test_extract_origin_invalid_url_fallback() {
        assert_eq!(extract_origin("not-a-url"), "http://localhost");
    }

    #[test]
    fn test_path_to_norns_path_simple_name_without_extension() {
        let path = PathBuf::from("awake");
        assert_eq!(path_to_norns_path(&path), "code/awake/awake.lua");
    }

    #[test]
    fn test_path_to_norns_path_simple_name_with_extension() {
        let path = PathBuf::from("awake.lua");
        assert_eq!(path_to_norns_path(&path), "code/awake/awake.lua");
    }

    #[test]
    fn test_path_to_norns_path_simple_name_different_script() {
        let path = PathBuf::from("mlr");
        assert_eq!(path_to_norns_path(&path), "code/mlr/mlr.lua");
    }

    #[test]
    fn test_path_to_norns_path_with_path_passes_through() {
        let path = PathBuf::from("dust/code/myproject/script.lua");
        assert_eq!(path_to_norns_path(&path), "dust/code/myproject/script.lua");
    }

    #[test]
    fn test_path_to_norns_path_absolute_path_passes_through() {
        let path = PathBuf::from("/home/user/dust/code/awake/awake.lua");
        assert_eq!(path_to_norns_path(&path), "/home/user/dust/code/awake/awake.lua");
    }

    #[test]
    fn test_path_to_norns_path_relative_path_passes_through() {
        let path = PathBuf::from("../dust/code/script.lua");
        assert_eq!(path_to_norns_path(&path), "../dust/code/script.lua");
    }

    #[test]
    fn test_path_to_norns_path_code_path_passes_through() {
        let path = PathBuf::from("code/awake/awake.lua");
        assert_eq!(path_to_norns_path(&path), "code/awake/awake.lua");
    }

    #[test]
    fn test_path_to_norns_path_subdirectory_passes_through() {
        let path = PathBuf::from("myproject/lib/engine.lua");
        assert_eq!(path_to_norns_path(&path), "myproject/lib/engine.lua");
    }
}
