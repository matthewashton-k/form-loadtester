use std::time::Duration;
use form_loadtester::{Args, Commands, crtsh::Scraper, Parameter, spammer::Sender};
use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = Args::parse();
    match cli.command {
        Commands::ScrapeCRTSH { domain, out} => {
            let mut scraper = Scraper::new(&domain);
            scraper.scrape_crt(out).await.expect("failed to scrape domains");
        }
        Commands::GetUpDomains {domain, out} => {
            let mut scraper = Scraper::new(&domain);
            scraper.scrape_crt(out).await.expect("failed to scrape domains");
            let mut rx = scraper.get_up_domains().await;
            
            while let Some(domain) = rx.recv().await {
                println!("{domain}");
            }
        }
        Commands::Spam { domain, max_open, config} => {
            let params: &'static mut Vec<Parameter> = Box::leak(Box::new(Parameter::get_params_from_config(&config).unwrap()));
            let form_builder = ||  {
                let params = Parameter::gen_param_map(params);
                params
            };
            let sender = Sender::new(Duration::from_secs(20), &domain, form_builder, max_open).unwrap();
            sender.begin().await.unwrap();
        },
    }
}
