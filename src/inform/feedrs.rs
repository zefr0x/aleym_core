use std::str::FromStr;

use super::{InformantError, utils};
use crate::db::{InputNews, time::OffsetDateTime};
use crate::net::{
	self,
	protocols::http::{self, body_util::BodyExt},
};

pub(crate) struct Informant {
	network: net::Client,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Parameters {
	/// # Feed URL
	#[schemars(url)]
	pub feed_url: String,
}

impl Informant {
	async fn fetch(&self, target: http::Uri) -> Result<http::Response<http::body::Incoming>, InformantError> {
		#[cfg(not(feature = "_net_protocol_http"))]
		compile_error!("HTTP protocol is required to be enabled for compiling with `informant_feedrs`");

		let response = self
			.network
			.http_request(
				http::Request::builder()
					.header(
						http::header::HOST,
						target.authority().ok_or(InformantError::NoTargetUriAuthority)?.as_str(),
					)
					.uri(&target)
					.method(http::Method::GET)
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

	fn parse(body: &[u8]) -> Result<Vec<InputNews>, InformantError> {
		// NOTE: This parser isn't thread safe, so we can't initiate it in the constructor of Self.
		let feed = feed_rs::parser::Builder::new()
			.sanitize_content(true)
			.build()
			.parse(body)?;

		let mut items = vec![];

		for entiry in feed.entries {
			items.push(InputNews {
				// TODO: Handle more data when the database is fully generalized.
				source_provided_id: Some(entiry.id),
				uri: entiry.links.first().map(|link| link.href.to_owned()),
				title: entiry.title.map(|title| title.content).unwrap_or_default(),
				summary: entiry.summary.map(|summary| summary.content),
				content: entiry.content.map(|content| content.body.unwrap_or_default()),
				published_at: entiry
					.published
					.map(|t| OffsetDateTime::from_unix_timestamp(t.timestamp()).unwrap()),
				updated_at: entiry
					.updated
					.map(|t| OffsetDateTime::from_unix_timestamp(t.timestamp()).unwrap()),
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
		let response_bytes = self
			.fetch(http::Uri::from_str(&parameters.feed_url)?)
			.await?
			// TODO: Pass the response to the parser as a stream if possible.
			.collect()
			.await
			.map_err(net::NetworkError::from)?
			.to_bytes();

		// TODO: Sandbox the parsing operation.
		utils::spawn_cpu_blocking(move || Self::parse(&response_bytes)).await?
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const EXAMPLE_RSS: &[u8] = include_bytes!("../../test/assets/example_rss_feed.rss");

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn parsing_rss() {
		let news = Informant::parse(&http::body::Bytes::from_static(EXAMPLE_RSS)).unwrap();

		assert_eq!(news.len(), 5);

		let first = news.first().unwrap();
		let last = news.last().unwrap();

		assert_eq!(
			&first.title,
			"Louisiana Students to Hear from NASA Astronauts Aboard Space Station"
		);
		assert_eq!(
			first.uri,
			Some("http://www.nasa.gov/press-release/louisiana-students-to-hear-from-nasa-astronauts-aboard-space-station".to_owned())
		);
		assert_eq!(
			first.summary,
			Some("As part of the state's first Earth-to-space call, students from Louisiana will have an opportunity soon to hear from NASA astronauts aboard the International Space Station.".to_owned())
		);
		assert_eq!(first.published_at, Some(time::macros::datetime!(2023-07-21 9:04 -4)));
		assert_eq!(
			first.source_provided_id,
			Some("http://www.nasa.gov/press-release/louisiana-students-to-hear-from-nasa-astronauts-aboard-space-station".to_owned())
		);

		assert_eq!(
			&last.title,
			"NASA Plans Coverage of Roscosmos Spacewalk Outside Space Station"
		);
		assert_eq!(
			last.uri,
			Some("http://liftoff.msfc.nasa.gov/news/2003/news-laundry.asp".to_owned())
		);
		assert_eq!(
			last.summary,
			Some("Compared to earlier spacecraft, the International Space Station has many luxuries, but laundry facilities are not one of them.  Instead, astronauts have other options.".to_owned())
		);
		assert_eq!(last.published_at, Some(time::macros::datetime!(2023-6-26 12:45 -4)));
		assert_eq!(
			last.source_provided_id,
			Some("http://liftoff.msfc.nasa.gov/2003/05/20.html#item570".to_owned())
		);
	}
}
