use std::collections::HashMap;
use std::option::Option;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use percent_encoding::percent_decode_str;
use reqwest::header::CONTENT_TYPE;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct SitePeekCache {
	cache: Mutex<HashMap<String, AnalyzeResult>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnalyzeResult {
	Unknown,
	// Image,Video link, currently same as input, but might change later
	Image(String),
	Video(String),
	Site(AnalyzeResultSite),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyzeResultSite {
	title: String,
	image_src: String,
	description: Option<String>,
}

impl SitePeekCache {
	pub async fn decode_and_analyze_link(&self, link: &str) -> AnalyzeResult {
		{
			let cache = self.cache.lock().unwrap();
			if let Some(cached_value) = cache.get(link) {
				return cached_value.clone();
			}
		}
		if let Ok(url) = percent_decode_str(link).decode_utf8() {
			let result = Self::analyze_link(url.as_ref()).await.unwrap_or(AnalyzeResult::Unknown);
			{
				let mut cache = self.cache.lock().unwrap();
				cache.insert(link.to_string(), result.clone());
			}
			result
		} else {
			AnalyzeResult::Unknown
		}
	}

	async fn analyze_link(link: &str) -> Result<AnalyzeResult> {
		let response = reqwest::get(link).await?;
		let headers = response.headers();
		let content_type =
			headers.get(CONTENT_TYPE).ok_or_else(|| anyhow!("No content type"))?.to_str()?;
		if content_type.starts_with("image/") {
			Ok(AnalyzeResult::Image(link.to_string()))
		} else if content_type.starts_with("video/") {
			Ok(AnalyzeResult::Video(link.to_string()))
		} else if content_type.starts_with("text/html") {
			let document_str = response.text().await?;
			let document = Html::parse_document(&document_str);

			// TODO also add reader for
			// - <meta name="Description" content="Phoronix is the leading technology …">
			// - <title>Open-Source RADV Vulkan Driver Is Seeing Work To … - Phoronix</title>
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
			Ok(AnalyzeResult::Site(AnalyzeResultSite { title, image_src, description }))
		} else {
			Ok(AnalyzeResult::Unknown)
		}
	}
}
