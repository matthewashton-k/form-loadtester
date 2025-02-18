use std::{path::PathBuf, time::Duration, net::ToSocketAddrs};

use reqwest::{Client, redirect::Policy};
use scraper::Selector;
use tokio::{sync::mpsc::Receiver, time::timeout, net::TcpStream};

#[derive(Debug)]
pub struct Scraper<'a> {
    domain: &'a str,
    records: Vec<String>,
}

impl<'a> Scraper<'a> {
    pub fn new(domain: &'a str) -> Self {
        Self{
            domain,
            records: Vec::new(),
        }
    }
    pub async fn scrape_crt(&mut self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let body = reqwest::get(format!("https://crt.sh/?q={}", self.domain)).await?.text().await?;

        let html = scraper::Html::parse_document(&body);
        let outer_table = html.select(&Selector::parse("td.outer")?).nth(1).ok_or("outer table not found")?;
        let table_body = outer_table.select(&Selector::parse("table")?).next().ok_or("table not found")?;
        let table_body = outer_table.select(&Selector::parse("tbody")?).next().ok_or("table not found")?;
        let mut csv_file = csv::WriterBuilder::default().from_path(path)?;
    
        for row in table_body.select(&Selector::parse("tr")?) {
            let selector = Selector::parse("th")?;
            let headers: Vec<&str> = row.select(&selector).map(|header| {
                let text = header.text().next().ok_or("no header value");
                text
            }).filter_map(|item| {
                item.ok()
            }).collect();
            if headers.len() > 0 {
                csv_file.write_record(headers)?;
            }
            
            let selector = Selector::parse("td")?;
            let data = row.select(&selector);
            let entries: Vec<String> = data.map(|data|{
                let text = data.text().next().map(|val| val.to_owned()).unwrap_or_else(|| data.inner_html());
                self.records.sort();
                self.records.dedup();
                if text.contains(self.domain) {
                    self.records.push(text.to_owned());
                }
                text
            }).collect();
            
            if entries.len() > 0 {
                csv_file.write_record(entries)?;
            }
        }
        self.records.sort();
        self.records.dedup();
        csv_file.flush()?;
        return Ok(());
    }
    
    pub async fn get_up_domains(&self) -> Receiver<String> {
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(self.records.len());
        for url in &self.records {
            let tx = tx.clone();
            let url = url.to_string();
            
            tokio::spawn(async move {
                if Self::check_up(&url).await {
                    let _ = tx.send(url).await;
                } else {
                }
            });
        }
        
        // Drop the original sender so the channel closes when all checks complete
        drop(tx);
        rx
    }
    
    pub async fn check_up(url: &str) -> bool {
        let mut clean_domain: String = url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/').to_owned();
        
        clean_domain.push_str(":80");
        // First try to resolve the domain
        let addr_iter = match clean_domain.to_socket_addrs() {
            Ok(iter) => iter,
            Err(e) => {
                return false
            },
        };
        
            // Get the first resolved IP
        let addr = match addr_iter.into_iter().next() {
            Some(a) => {
                a
            },
            None => {
                return false
            },
        };
        
        // Try to establish a TCP connection with a timeout
        match timeout(
            Duration::from_secs(10),
            TcpStream::connect(addr),
        ).await {
            Ok(Ok(_)) => true,
            _ => {
                false
            },
        }
    }
   
}
