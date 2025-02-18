pub mod scraper;
pub mod spammer;
mod parser;
use crate::parser::parse_line;

pub use self::scraper as crtsh;
pub use self::parser::Parameter;

use std::{path::PathBuf, collections::HashMap, fs};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    
    /// scrape records from crt.sh and save them to a csv file
    ScrapeCRTSH {
        /// domain to lookup
        #[arg(short, long)]
        domain: String,
        
        /// where to save data scraped from crt.sh
        #[arg(short, long, )]
        out: PathBuf
    },
    
    /// scrape records from crt.sh and save them to a csv file, and print out domains that respond to a ping
    GetUpDomains {
        /// domain to lookup
        #[arg(short, long)]
        domain: String,
        
        /// where to save data scraped from crt.sh
        #[arg(short, long, )]
        out: PathBuf
    },
    
    /// spam a domain 
    Spam {
        /// domain to spam
        #[arg(short, long)]
        domain: String,
        
        /// config file specifying how parameters are to be generated
        #[arg(short, long)]
        config: String,
        
        /// max number of open requests to domain
        #[arg(short, long)]
        max_open: usize
    }
}

use rand::{Rng, rng, prelude::SliceRandom};

impl Parameter {
    fn gen_params(&self) -> HashMap<String, String> {
        let mut rng = rng();
        let mut params = HashMap::new();

        match self.clone() {
            Parameter::Email { name, domains } => {
                let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
                let len = rng.random_range(5..15);
                let username: String = (0..len)
                    .map(|_| chars[rng.random_range(0..chars.len())])
                    .collect();
                let domain = domains[rng.random_range(0..domains.len())].clone();
                params.insert(name.clone(), format!("{}@{}", username, domain));
            }

            Parameter::YesNo { name } => {
                params.insert(name.clone(), if rng.random_bool(0.5) { "Yes" } else { "No" }.to_string());
            }

            Parameter::CellPhone { name } => {
                let area_code = rng.random_range(100..999);
                let middle = rng.random_range(100..999);
                let end = rng.random_range(1000..9999);
                params.insert(name.clone(), format!("({}) {}-{}", area_code, middle, end));
            }

            Parameter::Date {name, min, max} => {
                let year = rng.random_range(min..=max);
                let month = rng.random_range(1..=12);
                let max_day = match month {
                    2 => if year % 4 == 0 { 29 } else { 28 },
                    4 | 6 | 9 | 11 => 30,
                    _ => 31
                };
                let day = rng.random_range(1..=max_day);
                
                params.insert(name.clone(), format!("{:02}/{:02}/{:04}", month, day, year));
            }

            Parameter::CheckBoxes { kvps } => {
                for (name, value) in kvps {
                    if rng.random_bool(0.5) {
                        params.insert(name.clone(), value.clone());
                    }
                }
            }

            Parameter::String { name, max_len } => {
                let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789., "
                    .chars()
                    .collect();
                let len = rng.random_range(1..=max_len);
                let random_string: String = (0..len)
                    .map(|_| chars[rng.random_range(0..chars.len())])
                    .collect();
                params.insert(name.clone(), random_string);
            }
            
            Parameter::Name{ name, max_len } => {
                let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
                    .chars()
                    .collect();
                let len = rng.random_range(1..=max_len);
                let first_name: String = (0..len)
                    .map(|_| chars[rng.random_range(0..chars.len())])
                    .collect();
                let last_name: String = (0..len)
                    .map(|_| chars[rng.random_range(0..chars.len())])
                    .collect();
                let full_name = format!("{first_name} {last_name}");
                params.insert(name.clone(), full_name); 
            }
            
            Parameter::OptionalString { name } => {
                if rng.random_bool(0.5) {
                    params.insert(name.clone(), String::new());
                }
            }
            Parameter::ChooseAny {mut options } => {
                options.shuffle(&mut rng);
                if options.len() > 0 {
                    let kvp = options[0].clone();
                    params.insert(kvp.0.clone(), kvp.1.clone());
                }
            },
            Parameter::ChooseN { n, mut kvps } => {
                kvps.shuffle(&mut rng);
                for kvp in &kvps[0..n] {
                    params.insert(kvp.0.clone(), kvp.1.clone());
                }
            },
            Parameter::Static { name, val } => {
                params.insert(name.clone(), val.clone());
            },
        }
        params
    }
    
    fn try_parse(input: &str) -> Result<Self, std::io::Error> {
        match parse_line(input) {
            Ok(result) => {
                return Ok(result.1);
            },
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::Other, "Parsing error")),
        }
        
    }
    
    pub fn get_params_from_config(path: &str) -> Result<Vec<Parameter>, std::io::Error> {
        let contents: String = fs::read_to_string(path)?;
        
        let mut param_list = Vec::new();
        for line in contents.lines().filter_map(|item| {
            let item = item.trim();
            if item.is_empty() {
                None
            } else {
                Some(item)
            }
        }) {
            let param = Parameter::try_parse(line)?;
            param_list.push(param);
        }
        Ok(param_list)
    }
    
    pub fn gen_param_map(param_list: &Vec<Parameter>) -> HashMap<String,String> {
        let mut params = Vec::new();
        for p in param_list {
            params.push(p.gen_params());
        }
        params.into_iter().flatten().collect()
    }
}
