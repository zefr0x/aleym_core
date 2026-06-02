use std::str::FromStr;
use std::sync::LazyLock;

use time::{OffsetDateTime, format_description::well_known::Iso8601};

use super::{InformantError, utils};
use crate::db::InputNews;
use crate::net::{
	self,
	protocols::http::{self, body_util::BodyExt as _},
};

pub(crate) struct Informant {
	network: net::Client,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Parameters {
	/// # Channel Identifier
	#[schemars(pattern(r"^[a-zA-Z][a-zA-Z0-9_]{4,31}$"))]
	pub channel_id: String,
}

const TELEGRAM_WEB_ENDPOINT: &str = "https://t.me";

// Define selectors
static POST_SELECTOR: LazyLock<scraper::Selector> =
	LazyLock::new(|| scraper::Selector::parse(".tgme_widget_message.js-widget_message").unwrap());
static POST_TEXT_CONTENT_SELECTOR: LazyLock<scraper::Selector> =
	LazyLock::new(|| scraper::Selector::parse("div.tgme_widget_message_text.js-message_text").unwrap());
static POST_TIME_SELECTOR: LazyLock<scraper::Selector> = LazyLock::new(|| {
	scraper::Selector::parse(
		"div.tgme_widget_message_footer.js-message_footer .tgme_widget_message_meta .tgme_widget_message_date time.time",
	)
	.unwrap()
});

impl Informant {
	async fn fetch(&self, channel_id: &str) -> Result<http::Response<http::body::Incoming>, InformantError> {
		#[cfg(not(feature = "_net_protocol_http"))]
		compile_error!("HTTP protocol is required to be enabled for compiling with `informant_telegram_web`");

		// TODO: When supported by our database, handle it with message indexes rather than just fetching latest batch.
		let target = http::Uri::from_str(&format!("{TELEGRAM_WEB_ENDPOINT}/s/{channel_id}"))?;

		let response = self
			.network
			.http_request(
				http::Request::builder()
					.header(http::header::CONTENT_LENGTH, 0) // Avoid error 411 (Length Required)
					.header("X-Requested-With", "XMLHttpRequest") // Only fetch message fragments without the whole HTML page
					.uri(&target)
					.method(http::Method::POST)
					.body(http::body_util::Empty::<http::body::Bytes>::new())
					.unwrap(),
				false,
			)
			.await?;

		// Convert failure status codes into errors.
		match response.status() {
			code if code.is_success() => Ok(response),
			code => Err(InformantError::NetworkError(
				net::NetworkError::UnsuccessfulHttpRequest(code),
			)),
		}
	}

	/// `channel_id` is used for validation purposes only.
	fn parse(body: &str, channel_id: &str) -> Result<Vec<InputNews>, InformantError> {
		use scraper::Html;

		// Unescape JavaScript styled string
		let body = unescaper::unescape(body)?;

		// Parse HTML fragment
		let fragment = Html::parse_fragment(&body);

		let mut items = vec![];

		// Iterate over each post
		for post in fragment.select(&POST_SELECTOR) {
			// Scrape Header Metadata

			// SECURITY: Ensure that it doesn't contain untrusted contents. It might be displayed or opened as a URL.
			let post_id = post
				.attr("data-post")
				.map(|s| s.to_owned())
				.ok_or(InformantError::TelegramWebUndefiedFormat)?;
			// Validate that id = `channel_id/post_id`,
			// where `channel_id` equals the fetch target and `post_id` is an integer.
			match post_id.split_once("/") {
				// TODO: Consider using `u16`.
				Some((c, p)) if c != channel_id || u32::from_str(p).is_err() => {
					Err(InformantError::TelegramWebUndefiedFormat)?
				}
				_ => {}
			}

			let post_url = format!("{TELEGRAM_WEB_ENDPOINT}/{post_id}");

			// Scrape Footer Metadata

			let post_time = OffsetDateTime::parse(
				post.select(&POST_TIME_SELECTOR)
					.next()
					.ok_or(InformantError::TelegramWebUndefiedFormat)?
					.attr("datetime") // ISO8601 formatted time string
					.ok_or(InformantError::TelegramWebUndefiedFormat)?,
				&Iso8601::DEFAULT,
			)?;

			// Scrape Post Contents

			let mut post_title = String::new();
			// Get the first parts of the message until a line break
			if let Some(element) = post.select(&POST_TEXT_CONTENT_SELECTOR).next() {
				for child in element.child_elements() {
					if child.value().name() == "br" {
						break;
					}
					post_title.push_str(&child.inner_html());
				}
			}

			let post_full_text_content = post
				.select(&POST_TEXT_CONTENT_SELECTOR)
				.next()
				.map(|element| element.inner_html());

			items.push(InputNews {
				// TODO: Handle more data when the database is fully generalized.
				source_provided_id: Some(post_id),
				uri: Some(post_url),
				// SECURITY: Clean HTML from untrusted contents.
				title: ammonia::clean(&post_title),
				summary: None,
				// SECURITY: Clean HTML from untrusted contents.
				content: post_full_text_content.map(|s| ammonia::clean(&s)),
				published_at: Some(post_time),
				updated_at: None,
			});
		}

		Ok(items)
	}
}

impl super::InformantTrait for Informant {
	type Parameters = Parameters;

	fn new(network_client: net::Client) -> Self {
		Self {
			network: network_client,
		}
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	async fn execute(self, parameters: Self::Parameters) -> Result<Vec<crate::db::InputNews>, InformantError> {
		tracing::trace!("fetching feed");

		let response_bytes = self
			.fetch(&parameters.channel_id)
			.await?
			// TODO: Pass the response to the parser as a stream if possible.
			.collect()
			.await
			.map_err(net::NetworkError::from)?;

		// TODO: Sandbox the parsing operation.
		utils::spawn_cpu_blocking(move || {
			tracing::trace!("parsing fetched feed");

			Self::parse(str::from_utf8(&response_bytes.to_bytes())?, &parameters.channel_id)
		})
		.await?
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const EXAMPLE_FEED: &[u8] = include_bytes!("../../test/assets/example_telegram_channel_feed.html");

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn parsing_telegram_web() {
		let news = Informant::parse(
			str::from_utf8(&http::body::Bytes::from_static(EXAMPLE_FEED)).unwrap(),
			"telegram",
		)
		.unwrap();

		assert_eq!(news.len(), 20);

		let first = news.first().unwrap();
		let last = news.last().unwrap();

		dbg!(first, last);

		assert_eq!(
			&first.title,
			"New Design.fully redesigned interface even quickermore responsive"
		);
		assert_eq!(first.uri, Some("https://t.me/telegram/425".to_owned()));
		assert_eq!(first.summary, None,);
		assert_eq!(
			first.published_at,
			Some(time::macros::datetime!(2026-02-10 17:43:45.0 +00))
		);
		assert_eq!(first.source_provided_id, Some("telegram/425".to_owned()));

		assert_eq!(
			&last.title,
			"statisticscustom limitsstreaming textsilent scheduled messages"
		);
		assert_eq!(last.uri, Some("https://t.me/telegram/445".to_owned()));
		assert_eq!(last.summary, None,);
		assert_eq!(
			last.published_at,
			Some(time::macros::datetime!(2026-05-14 16:09:50.0 +00))
		);
		assert_eq!(last.source_provided_id, Some("telegram/445".to_owned()));
	}
}
