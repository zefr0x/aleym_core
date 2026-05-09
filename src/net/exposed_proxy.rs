use std::sync::Arc;

use socks5_server::{
	Command, IncomingConnection, Server,
	auth::NoAuth,
	connection::state::NeedAuthenticate,
	proto::{Address, Error, Reply},
};
use tokio::io::AsyncWriteExt;

use super::{Network, NetworkError};

// Source: https://github.com/EAimTY/socks5-server/blob/df6d31c4e54128a7b9c319396cfa1ef06918d90a/socks5-server/examples/simple_socks5.rs
impl Network {
	/// Run a SOCKS5 proxy listener for the Tor network.
	pub async fn run_tor_socks5_proxy(&self, address: impl tokio::net::ToSocketAddrs) -> Result<(), NetworkError> {
		let listener = tokio::net::TcpListener::bind(address).await?;
		let auth = Arc::new(NoAuth) as Arc<_>;

		let server = Server::new(listener, auth);

		while let Ok((conn, _)) = server.accept().await {
			let tor_client = self.tor_client.isolated_client();

			tokio::spawn(async move {
				match Self::handle_connection(tor_client, conn).await {
					Ok(()) => {}
					Err(error) => tracing::error!(?error),
				}
			});
		}

		Ok(())
	}

	async fn handle_connection(
		tor_client: arti_client::TorClient<tor_rtcompat::PreferredRuntime>,
		conn: IncomingConnection<(), NeedAuthenticate>,
	) -> Result<(), Error> {
		let conn = match conn.authenticate().await {
			Ok((conn, _)) => conn,
			Err((err, mut conn)) => {
				conn.shutdown().await?;

				return Err(err);
			}
		};

		match conn.wait().await {
			Ok(Command::Associate(associate, _)) => {
				let replied = associate
					.reply(Reply::CommandNotSupported, Address::unspecified())
					.await;

				let mut conn = match replied {
					Ok(conn) => conn,
					Err((err, mut conn)) => {
						conn.shutdown().await?;

						return Err(Error::Io(err));
					}
				};

				conn.close().await?;
			}
			Ok(Command::Bind(bind, _)) => {
				let replied = bind.reply(Reply::CommandNotSupported, Address::unspecified()).await;

				let mut conn = match replied {
					Ok(conn) => conn,
					Err((err, mut conn)) => {
						conn.shutdown().await?;

						return Err(Error::Io(err));
					}
				};

				conn.close().await?;
			}
			Ok(Command::Connect(connect, addr)) => {
				let target = match addr {
					Address::DomainAddress(domain, port) => {
						let domain = String::from_utf8_lossy(&domain);
						tor_client.connect((domain.as_ref(), port)).await
					}
					Address::SocketAddress(addr) => tor_client.connect((addr.ip().to_string(), addr.port())).await,
				};

				if let Ok(mut target) = target {
					let replied = connect.reply(Reply::Succeeded, Address::unspecified()).await;

					let mut conn = match replied {
						Ok(conn) => conn,
						Err((err, mut conn)) => {
							conn.shutdown().await?;

							return Err(Error::Io(err));
						}
					};

					let res = tokio::io::copy_bidirectional(&mut target, &mut conn).await;

					conn.shutdown().await?;
					target.shutdown().await?;

					res?;
				} else {
					let replied = connect.reply(Reply::HostUnreachable, Address::unspecified()).await;

					let mut conn = match replied {
						Ok(conn) => conn,
						Err((err, mut conn)) => {
							conn.shutdown().await?;

							return Err(Error::Io(err));
						}
					};

					conn.shutdown().await?;
				}
			}
			Err((err, mut conn)) => {
				conn.shutdown().await?;

				return Err(err);
			}
		}

		Ok(())
	}
}
