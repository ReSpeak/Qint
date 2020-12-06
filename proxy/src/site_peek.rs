use std::collections::HashMap;
use std::option::Option;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use percent_encoding::percent_decode_str;
use reqwest::header::CONTENT_TYPE;
use scraper::{ElementRef, Html, Selector};
use serde::Serialize;

#[derive(Debug, Default)]
pub struct SitePeekCache {
	cache: Mutex<HashMap<String, AnalyzeResult>>,
}

#[derive(Clone, Debug, Serialize)]
pub enum AnalyzeResult {
	Unknown,
	// Image,Video link, currently same as input, but might change later
	Image(String),
	Video(String),
	Site { title: String, image_src: String, description: Option<String> },
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
		// Include 'Bot' in the user agent to be able to load Twitter previews
		// (https://mau.dev/maunium/synapse)
		let client = reqwest::Client::builder().user_agent("Bot").build()?;
		let response = client.get(link).send().await?;
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
			let head = document
				.select(&Selector::parse("head").unwrap())
				.next()
				.ok_or_else(|| anyhow!("No head"))?;

			// Special filtering for video-only content
			if let Some(og_type) = Self::try_select(&head, "meta[property='og:type']", "content") {
				if og_type.starts_with("video") {
					if let Some(video_url) =
						Self::try_select(&head, "meta[property='og:video']", "content")
					{
						return Ok(AnalyzeResult::Video(video_url.to_string()));
					}
				}
			}

			// Special filtering for curated sites
			if link.starts_with("https://imgur.com/") {
				if let Some(img_url) =
					Self::try_select(&head, "meta[name='twitter:image']", "content")
				{
					return Ok(AnalyzeResult::Image(img_url.to_string()));
				}
			}

			// General (OG) analysis
			let title = Self::try_select(&head, "meta[property='og:title']", "content")
				.or_else(|| {
					head.select(&Selector::parse("title").unwrap())
						.next()
						.and_then(|e| e.text().next())
				})
				.ok_or_else(|| anyhow!("No title"))?;
			let image_src = Self::try_select(&head, "meta[property='og:image']", "content")
				.or_else(|| {
					let favicon = Selector::parse("link[rel='icon']").unwrap();
					document
						.select(&favicon)
						.filter_map(|elem| {
							let elem = elem.value();
							elem.attr("href").map(|link| {
								(
									link,
									elem.attr("sizes")
										.and_then(|sizes| {
											sizes.find('x').and_then(|index| {
												sizes[..index].parse::<u32>().ok()
											})
										})
										.unwrap_or_default(),
								)
							})
						})
						.max_by_key(|(_, size)| *size)
						.map(|(link, _)| link)
				})
				.ok_or_else(|| anyhow!("No image"))?;

			let description = Self::try_select(&head, "meta[property='og:description']", "content")
				.or_else(|| Self::try_select(&head, "meta[name='description']", "content"));
			Ok(AnalyzeResult::Site {
				title: title.to_string(),
				image_src: image_src.to_string(),
				description: description.map(|e| e.to_string()),
			})
		} else {
			Ok(AnalyzeResult::Unknown)
		}
	}

	fn try_select<'a>(head: &'a ElementRef, selector: &str, attr: &str) -> Option<&'a str> {
		let selector = Selector::parse(selector).unwrap();
		head.select(&selector).next().and_then(|e| e.value().attr(attr))
	}
}
