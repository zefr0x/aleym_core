use url::Url;

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
	async fn fetch(&self, mut target: Url) -> Result<http::Response<http::body::Incoming>, InformantError> {
		#[cfg(not(feature = "_net_protocol_http"))]
		compile_error!("HTTP protocol is required to be enabled for compiling with `informant_feedrs`");

		const REDIRECTION_CHAIN_LIMIT: u8 = match option_env!("FEEDRS_HTTP_REDIRECTION_CHAIN_LIMIT") {
			Some(v) => const_str::parse!(v, u8),
			None => 7, // Default
		};
		let mut redirection_chain_counter = 0;

		tracing::debug!(?target, "initiating fetch");

		loop {
			let response = self
				.network
				.http_request(
					http::Request::builder()
						.header(http::header::HOST, target.authority())
						.uri(target.as_str())
						.method(http::Method::GET)
						.body(http::body_util::Empty::<http::body::Bytes>::new())
						.unwrap(),
					// TODO: Expose HTTP/2 prior knowledge as a parameter.
					false,
				)
				.await?;

			// TODO: Split redirection handlers to utility functions.
			match response.status() {
				// Check if response is success
				code if code.is_success() => return Ok(response),
				// Handle permanent redirection (currently treated as temporary)
				http::StatusCode::MOVED_PERMANENTLY
				| http::StatusCode::PERMANENT_REDIRECT
				// TODO: Move temporary redirection handler to generic lower level implementation.
				// Handle temporary redirection
				| http::StatusCode::FOUND
				| http::StatusCode::TEMPORARY_REDIRECT => {
					// VULN: This implementation doesn't detect circular redirection loops, but only limits the count.
					// SECURITY: Capping the redirection count to avoid infinite redirection attacks.
					if redirection_chain_counter >= REDIRECTION_CHAIN_LIMIT {
						return Err(InformantError::HttpRedirectionCountLimitReached); // Stop the redirection loop
					} else if let Some(location) = response.headers().get(http::header::LOCATION)
						&& !location.is_empty()
					{
						let location = location.to_str()?;

						match Url::parse(location) {
							// When the location is absolute, and has an authority
							Ok(redirection_url) if redirection_url.has_authority() => {
								tracing::debug!(?redirection_url, "handling absolute location redirection");

								// Only allow upgrading scheme from HTTP to HTTPS
								match (target.scheme(), redirection_url.scheme()) {
									(from, to) if from == to || (from == "http" && to == "https") => {
										target = redirection_url;
									},
									(from, to) => {
										return Err(InformantError::UnallowedRedirectionSchemeChange { from: from.to_owned(), to: to.to_owned() });
									}
								}
							}
							// When the location is absolute, but doesn't have an authority
							Ok(redirection_url) => {
								return Err(InformantError::InvalidHttpRedirectionLocation(redirection_url));
							}
							// When the location is relative
							Err(url::ParseError::RelativeUrlWithoutBase) => {
								tracing::debug!(?location, "handling relative location redirection");

								// Relatively join the new path with the old one
								target = target.join(location)?;
							}
							Err(error) => Err(error)?
						}
					} else {
						return Err(
							InformantError::NoHttpRedirectionLocation,
						);
					}

					redirection_chain_counter += 1;
					tracing::debug!(redirection_chain_counter, new_target=?target, "proceeding with temporary redirection to another target");
				}
				// Convert other status codes into errors
				code => {
					return Err(InformantError::NetworkError(
						net::NetworkError::UnsuccessfulHttpRequest(code),
					));
				}
			}
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
			.fetch(Url::parse(&parameters.feed_url)?)
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
