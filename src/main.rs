use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use url::Url;

async fn fetch_links(url: &str) -> Vec<String> {
    let client = Client::new();
    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Failed to fetch links from {}: {}", url, e);
            return Vec::new();
        }
    };
    let body = match response.text().await {
        Ok(body) => body,
        Err(e) => {
            eprintln!("Failed to read response body from {}: {}", url, e);
            return Vec::new();
        }
    };
    let document = Html::parse_document(&body);

    let selector = Selector::parse("a[href]").unwrap();
    let mut links = Vec::new();

    for link in document.select(&selector) {
        let href = link.value().attr("href").unwrap_or("").to_string();
        if let Ok(parsed_url) = Url::parse(&href) {
            if parsed_url.scheme() != "mailto" {
                links.push(href);
            }
        }
    }

    println!("Fetched {} links from {}", links.len(), url);
    links
}

fn process_links(links: Vec<String>, base_url: &str) -> Vec<String> {
    let mut processed_links = Vec::new();
    for link in links {
        if let Ok(absolute_url) = Url::parse(base_url).unwrap().join(&link) {
            processed_links.push(absolute_url.to_string());
        }
    }
    processed_links
}

fn get_emails(html: &str) -> Vec<String> {
    let email_regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    let mut emails = HashSet::new();

    for email in email_regex.find_iter(html) {
        emails.insert(email.as_str().to_string());
    }

    emails.into_iter().collect()
}

fn get_phones(links: &[String]) -> Vec<String> {
    let mut phones = HashSet::new();

    for link in links {
        if link.starts_with("tel:") {
            phones.insert(link.replace("tel:", ""));
        }
    }

    phones.into_iter().collect()
}

#[tokio::main]
async fn main() {
    let url = "http://legionerror.wordpress.com";
    let initial_links = fetch_links(url).await;
    let processed_links = process_links(initial_links, url);

    let mut all_links = HashSet::new();
    all_links.extend(processed_links.clone());

    let mut all_html = String::new();

    for link in &processed_links {
        let links = fetch_links(link).await;
        all_links.extend(process_links(links, link));

        let response = match Client::new().get(link).send().await {
            Ok(response) => response,
            Err(e) => {
                eprintln!("Failed to fetch HTML from {}: {}", link, e);
                continue;
            }
        };
        let html = match response.text().await {
            Ok(html) => html,
            Err(e) => {
                eprintln!("Failed to read response body from {}: {}", link, e);
                continue;
            }
        };
        all_html.push_str(&html);
    }

    let emails = get_emails(&all_html);
    let phones = get_phones(&all_links.iter().cloned().collect::<Vec<_>>());

    let data = HashMap::from([("emails", emails), ("phone_numbers", phones)]);

    println!("Data after crawling: {:?}", data);
}
