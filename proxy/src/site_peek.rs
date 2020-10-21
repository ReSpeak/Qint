use std::collections::HashMap;
use std::option::Option;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use percent_encoding::percent_decode_str;
use reqwest::header::CONTENT_TYPE;
use scraper::{Html, Selector};

// TODO maybe not gud as global state object
// Perhaps add it to our app state and pass it to the func?
lazy_static! {
	static ref CACHE: Mutex<HashMap<String, AnalyzeResult>> = Mutex::new(HashMap::new());
}

pub(crate) async fn decode_and_analyze_link(link: &str) -> AnalyzeResult {
	{
		let cache = CACHE.lock().unwrap();
		if let Some(cached_value) = cache.get(link) {
			return cached_value.clone();
		}
	}
	if let Ok(url) = percent_decode_str(link).decode_utf8() {
		let result = analyze_link(url.as_ref()).await.unwrap_or(AnalyzeResult::Unknown);
		{
			let mut cache = CACHE.lock().unwrap();
			cache.insert(link.to_string(), result.clone());
		}
		result
	} else {
		AnalyzeResult::Unknown
	}
}

pub(crate) async fn analyze_link(link: &str) -> Result<AnalyzeResult> {
	if let Ok(response) = reqwest::get(link).await {
		let headers = response.headers();
		let content_type =
			headers.get(CONTENT_TYPE).ok_or_else(|| anyhow!("No content type"))?.to_str()?;
		if content_type.starts_with("image/") {
			return Ok(AnalyzeResult::Image(link.to_string()));
		} else if content_type.starts_with("video/") {
			return Ok(AnalyzeResult::Video(link.to_string()));
		} else if content_type.starts_with("text/html") {
			let document_str = response.text().await?;
			let document = Html::parse_document(&document_str);

			// TODO also add reader for
			// - <meta name="Description" content="Phoronix is the leading technology website for Linux hardware reviews, open-source news, Linux benchmarks, open-source benchmarks, and computer hardware tests.">
			// - <title>Open-Source RADV Vulkan Driver Is Seeing Work To Allow Building It On Windows - Phoronix</title>
			// - <link rel="icon" type="image/png" href="/android-chrome-192x192.png" sizes="192x192"> (pick biggest size)
			let selector_title = Selector::parse(r#"meta[property='og:title']"#).unwrap();
			let selector_image = Selector::parse(r#"meta[property='og:image']"#).unwrap();
			let selector_descr = Selector::parse(r#"meta[property='og:description']"#).unwrap();

			let title = document
				.select(&selector_title)
				.next()
				.and_then(|e| e.value().attr("content"))
				.map(|e| e.to_string())
				.ok_or_else(|| anyhow!("No title"))?;
			let image_src = document
				.select(&selector_image)
				.next()
				.and_then(|e| e.value().attr("content"))
				.map(|e| e.to_string())
				.ok_or_else(|| anyhow!("No image"))?;
			let description = document
				.select(&selector_descr)
				.next()
				.and_then(|e| e.value().attr("content"))
				.map(|e| e.to_string());
			return Ok(AnalyzeResult::Site(AnalyzeResultSite { title, image_src, description }));
		}
	}
	Ok(AnalyzeResult::Unknown)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum AnalyzeResult {
	Unknown,
	// Image,Video link, currently same as input, but might change later
	Image(String),
	Video(String),
	Site(AnalyzeResultSite),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AnalyzeResultSite {
	title: String,
	image_src: String,
	description: Option<String>,
}
