use serde_json;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub amqp_urls: Vec<String>,
    pub duration: u64,
}

impl Config {
    pub fn load(path: &str) -> Self {
        let config_file = File::open(path).unwrap();
        let buf = BufReader::new(config_file);
        let config: Config = serde_json::from_reader(buf).expect("Failed to load config.");
        config
    }
}
