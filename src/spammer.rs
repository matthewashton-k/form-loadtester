use std::{collections::HashMap, sync::atomic::{AtomicU32, Ordering}, time::Duration};

use reqwest::{Client, ClientBuilder, redirect::Policy};
use tokio::{sync::{Semaphore, watch}, time::Instant};

pub struct Sender<T> 
where T: FnMut() -> HashMap<String, String> + Send + Sync + 'static {
    permits: Semaphore,
    sent: AtomicU32,
    failed: AtomicU32,
    form_builder: T,
    client: &'static mut Client,
    domain: String
}

impl<T> Sender<T> where T: Fn() -> HashMap<String, String> + Send + Sync + 'static {
    pub fn new(timeout: Duration, domain: &str, form_builder: T, max_open_requests: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Box::new(ClientBuilder::new().user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.5615.50 Safari/537.36"
        ).redirect(Policy::limited(5)).timeout(timeout).connect_timeout(timeout).build()?);
        Ok(Self {
            permits: Semaphore::new(max_open_requests),
            sent: AtomicU32::new(0),
            failed: AtomicU32::new(0),
            form_builder,
            client: Box::leak(client),
            domain: domain.to_owned()
        })
    }
    
    async fn send_request(&self) -> Result<(), Box<dyn std::error::Error>> {
        let params: HashMap<String, String> = (self.form_builder)();
        let mut form = reqwest::multipart::Form::new();
        for (key, val) in params.into_iter() {
            form = form.text(key, val);
        }
        let resp = self.client.post(&self.domain).multipart(form).send().await?;
        if resp.status().is_success() {
            self.sent.fetch_add(1, Ordering::SeqCst);
        } else {
            self.failed.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
    
    #[allow(unreachable_code)]
    pub async fn begin(self) -> Result<(), Box<dyn std::error::Error>> {
        let (ctrlc_tx, ctrlc_rx) = watch::channel(false);
        ctrlc::set_handler(move || {
            ctrlc_tx.send(true).expect("failed to send ctrlc");
        })?;
        let self_ref = Box::leak(Box::new(self));
        let start = Instant::now();
        let mut timer = Instant::now();
        'main: loop {
            if *ctrlc_rx.borrow() {
                let elapsed = start.elapsed().as_secs_f64();
                println!("[*] {} requests set. {} failed requests. {} average requests per second.", 
                    self_ref.sent.load(Ordering::Relaxed), 
                    self_ref.failed.load(Ordering::Relaxed),
                    (self_ref.sent.load(Ordering::Relaxed) as f64)/elapsed);
                break 'main;
            }
            let permit = self_ref.permits.acquire().await;
            if timer.elapsed().as_secs() > 10 {
                let elapsed = start.elapsed().as_secs_f64();
                println!("[*] {} requests set. {} failed requests. {} average requests per second.", 
                    self_ref.sent.load(Ordering::Relaxed), 
                    self_ref.failed.load(Ordering::Relaxed),
                    (self_ref.sent.load(Ordering::Relaxed) as f64)/elapsed);
                timer = Instant::now();
            }
            tokio::spawn(async {
                self_ref.send_request().await.unwrap();
                drop(permit);
            });
        }
        Ok(())
    }
}
