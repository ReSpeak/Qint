use std::net::SocketAddr;
use std::sync::Arc;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::middleware::Condition;
use actix_web::web::{Data, Query};
use actix_web::*;

use actix_web::http::header::{CACHE_CONTROL, ETAG, HeaderValue};
use anyhow::Result;
use base64::prelude::*;
use futures::prelude::*;
use juniper::http::GraphQLRequest;
use juniper::http::graphiql::graphiql_source;
use qint_proxy::connection::{DownloadFileContext, UploadFileContext};
use qint_proxy::filecache::guess_content_type;
use qint_proxy::messages::ResultDetails;
use qint_proxy::{ConnectionId, QintState};
use rand::Rng;
use serde::Deserialize;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{debug, error, info, warn};
use tsclientlib::ChannelId;
use tsclientlib::Error as TsError;
use tsproto_types::crypto::EccKeyPubP256;

use crate::websocket::Ws;

pub struct WebApp {
	state: Arc<QintState>,
	/// Authentication token, this needs to be set in the qint-auth cookie.
	token: String,
}

impl WebApp {
	pub fn new(state: Arc<QintState>) -> Self {
		let mut rng = rand::thread_rng();
		let token = format!("{:0x}{:0x}", rng.gen::<u64>(), rng.gen::<u64>());

		Self { state, token }
	}

	pub async fn serve(self) -> Result<()> {
		let frontend_path = std::option_env!("FRONTEND_PATH").unwrap_or("../frontend/dist/");
		let is_production = std::option_env!("FRONTEND_PATH").is_some();
		info!(frontend_path, "Serving frontend");
		let state2 = self.state.clone();
		let addr = self.get_listen_address();
		let token = self.get_token().to_string();

		HttpServer::new(move || {
			let state = state2.clone();
			let token = token.clone();
			actix_web::App::new()
				//.wrap(middleware::Logger::default())
				// Return error messages
				.app_data(web::JsonConfig::default().error_handler(|err, _| {
					let err_string = err.to_string();
					error::InternalError::from_response(
						err, HttpResponse::BadRequest().body(err_string)).into()
				}))
				.wrap_fn(move |req, srv| {
					if is_production {
						if let Some(resp) = check_authentication(&token, &req) {
							return future::Either::Left(future::ok(ServiceResponse::new(req.into_parts().0, resp)));
						}
					}
					// Token is ok
					future::Either::Right(srv.call(req))
				})
				.wrap(Condition::new(!is_production, Cors::permissive().max_age(3600)))
				.app_data(Data::new(state))
				.service(create_main_ws)
				.service(audio_reset)
				.service(download_file)
				.service(upload_file)
				.service(download_cache_file)
				.service(db_graphql)
				.service(graphiql)
				.service(Files::new("", frontend_path).index_file("index.html"))
				.wrap_fn(|req, srv| {
					let fut = srv.call(req);
					async {
						let mut res = fut.await?;
						let headers = res.headers_mut();
						if headers.contains_key(ETAG) {
							headers.insert(
								CACHE_CONTROL,
								HeaderValue::from_static("no-cache,must-revalidate"),
							);
						}
						Ok(res)
					}
				})
		})
		.bind(addr)?
		.run()
		.await?;

		// Quit all connections
		info!("Closing remaining connections");
		self.state.close_all().await;

		Ok(())
	}

	pub fn get_listen_address(&self) -> SocketAddr {
		let settings = self.state.launch_config.read().unwrap();
		settings.listen_address
	}

	pub fn get_token(&self) -> &str { &self.token }
}

#[get("/ws")]
async fn create_main_ws(
	state: web::Data<Arc<QintState>>, req: HttpRequest, stream: web::Payload,
) -> impl Responder {
	let (response, session, mut msg_stream) = match actix_ws::handle(&req, stream) {
		Ok(r) => r,
		Err(error) => {
			error!(%error, "Failed to create websocket actor");
			return Either::Left(
				HttpResponse::InternalServerError().body("Failed to start connection"),
			);
		}
	};

	actix::spawn(async move {
		let mut webws = Ws::new((**state).clone(), session.clone());
		while let Some(msg) = msg_stream.recv().await {
			if let Err(error) = webws.on_msg(msg).await {
				error!(%error, "Failed to handle websocket message");
				break;
			}
		}

		webws.close();
		let _ = session.close(None).await;
	});

	Either::Right(response)
}

#[derive(Deserialize)]
struct GetFileOptions {
	dl: Option<String>,
	return_code: Option<String>,
	#[serde(default)]
	cache: bool,
}

fn result_details_gone() -> ResultDetails { "gone".into() }

#[get("/con/{id}/file/{channel}/{path:.*}")]
async fn download_file(
	state: web::Data<Arc<QintState>>, path: web::Path<(ConnectionId, u64, String)>,
	query_opt: Query<GetFileOptions>,
) -> impl Responder {
	let (id, channel, path) = path.into_inner();
	let path = format!("/{}", path);
	let channel = ChannelId(channel);
	let GetFileOptions { dl, return_code, cache } = query_opt.into_inner();

	let con = match state.get_connection(&id) {
		Some(con) => con,
		_ => {
			return HttpResponse::Gone().json(result_details_gone());
		}
	};

	// Lookup in cache
	let server = match con.send(qint_proxy::connection::GetPublicKeyMsg).await {
		Ok(Ok(r)) => r,
		Ok(Err(error)) => {
			error!(%error, "Failed to get server public key");
			return HttpResponse::Gone().json(result_details_gone());
		}
		Err(_) => {
			return HttpResponse::Gone().json(result_details_gone());
		}
	};

	let build_response = |len: u64, mime: Option<&str>| {
		let mut response = HttpResponse::Ok();
		response.no_chunking(len);
		if let Some(mime) = mime {
			response.content_type(mime);
		}
		if let Some(filename) = dl.as_ref() {
			response.insert_header((
				"Content-Disposition",
				format!("attachment; filename=\"{}\"", filename),
			));
		}
		response
	};

	if let Some((len, stream)) = state.file_cache.get_cached_file(&server, channel, &path).await {
		let (stream, mime) = guess_content_type(stream).await;
		let mut response = build_response(len, mime);
		return response.streaming(stream);
	}

	debug!(channel = channel.0, %path, "Downloading file");
	let DownloadFileContext { size, stream } = match con
		.send(qint_proxy::connection::DownloadFile {
			channel,
			path: path.clone(),
			return_code,
			channel_password: None,
			resume: false,
		})
		.await
	{
		Err(_) => {
			return HttpResponse::Gone().json(result_details_gone());
		}
		Ok(Err(qint_proxy::connection::Error::TsError(TsError::CommandError(error)))) => {
			debug!(%error, %path, "File download error");
			return match error.error {
				tsclientlib::TsError::FileInvalidPath => {
					HttpResponse::NotFound().json(Into::<ResultDetails>::into(error))
				}
				tsclientlib::TsError::PermissionsClientInsufficient => {
					HttpResponse::Forbidden().json(Into::<ResultDetails>::into(error))
				}
				_ => HttpResponse::BadRequest().json(Into::<ResultDetails>::into(error)),
			};
		}
		Ok(Err(error)) => {
			error!(%error, %path, "File download failed");
			return HttpResponse::InternalServerError()
				.json(ResultDetails::from_desc(format!("Failed to download file: {}", error)));
		}
		Ok(Ok(r)) => r,
	};

	let stream = FramedRead::new(stream, BytesCodec::new()).map(|r| r.map(web::BytesMut::freeze));
	let (stream, mime) = guess_content_type(stream).await;
	let mut response = build_response(size, mime);

	// Cache for offline usage if smaller than 5 MiB
	if cache && size < 5 * 1024 * 1024 {
		let stream = state.file_cache.cache_file(&server, channel, &path, stream).await;
		response.streaming(stream)
	} else {
		response.streaming(stream)
	}
}

#[derive(Deserialize)]
struct PutFileOptions {
	return_code: Option<String>,
}

#[put("/con/{id}/file/{channel}/{path:.*}")]
async fn upload_file(
	state: web::Data<Arc<QintState>>, path: web::Path<(ConnectionId, u64, String)>,
	req: HttpRequest, body: web::Payload, query_opt: Query<PutFileOptions>,
) -> impl Responder {
	let (id, channel, path) = path.into_inner();
	let path = format!("/{}", path);
	let channel = ChannelId(channel);

	let con = match state.get_connection(&id) {
		Some(con) => con,
		_ => {
			return HttpResponse::Gone().json(result_details_gone());
		}
	};

	debug!(channel = channel.0, %path, "Uploading file");
	let size = if let Some(r) = req.headers().get(http::header::CONTENT_LENGTH) {
		match r.to_str() {
			Err(error) => {
				warn!(%error, "Invalid content length header");
				return HttpResponse::BadRequest().body("Invalid content length header");
			}
			Ok(s) => match s.parse() {
				Err(error) => {
					warn!(%error, "Invalid content length header value");
					return HttpResponse::BadRequest()
						.body("Invalid content length header - not a number");
				}
				Ok(r) => r,
			},
		}
	} else {
		return HttpResponse::BadRequest().body("Content length header is missing");
	};
	let UploadFileContext { mut stream } = match con
		.send(qint_proxy::connection::UploadFile {
			channel,
			path: path.clone(),
			channel_password: None,
			size,
			overwrite: true,
			resume: false,
			return_code: query_opt.return_code.clone(),
		})
		.await
	{
		Err(_) => {
			return HttpResponse::Gone().json(result_details_gone());
		}
		Ok(Err(qint_proxy::connection::Error::TsError(TsError::CommandError(error)))) => {
			debug!(%error, %path, "File upload error");
			return match error.error {
				tsclientlib::TsError::FileInvalidPath => {
					HttpResponse::NotFound().json(Into::<ResultDetails>::into(error))
				}
				tsclientlib::TsError::PermissionsClientInsufficient => {
					HttpResponse::Forbidden().json(Into::<ResultDetails>::into(error))
				}
				_ => HttpResponse::BadRequest().json(Into::<ResultDetails>::into(error)),
			};
		}
		Ok(Err(error)) => {
			error!(%error, %path, "File upload failed");
			return HttpResponse::InternalServerError()
				.json(ResultDetails::from_desc(format!("Failed to upload file: {}", error)));
		}
		Ok(Ok(r)) => r,
	};
	// Upload
	let mut body_reader = tokio_util::io::StreamReader::new(body.map_err(|e| {
		std::io::Error::new(std::io::ErrorKind::Other, format!("Payload error {}", e))
	}));
	if let Err(error) = tokio::io::copy(&mut body_reader, &mut stream).await {
		warn!(%error, "File upload aborted");
		return HttpResponse::BadGateway()
			.json(ResultDetails::from_desc(format!("Upload failed: {}", error)));
	}
	HttpResponse::Ok().json(ResultDetails::ok())
}

/// Get a cached file by server id, channel and path.
#[get("/filecache/{id}/{channel}/{path:.*}")]
async fn download_cache_file(
	state: web::Data<Arc<QintState>>, path: web::Path<(String, u64, String)>,
) -> impl Responder {
	let (id, channel, path) = path.into_inner();
	let path = format!("/{}", path);
	let server = match BASE64_URL_SAFE_NO_PAD
		.decode(&id)
		.map_err(|e| e.into())
		.and_then(|id| EccKeyPubP256::from_short(&id))
	{
		Err(e) => {
			return HttpResponse::BadRequest().body(format!("Not a valid server id: {}", e));
		}
		Ok(id) => id,
	};
	let channel = ChannelId(channel);
	if let Some((len, stream)) = state.file_cache.get_cached_file(&server, channel, &path).await {
		let (stream, mime) = guess_content_type(stream).await;
		let mut response = HttpResponse::Ok();
		response.no_chunking(len);
		if let Some(mime) = mime {
			response.content_type(mime);
		}
		response.streaming(stream)
	} else {
		HttpResponse::NotFound().finish()
	}
}

#[post("/audio/reset")]
async fn audio_reset(state: web::Data<Arc<QintState>>) -> impl Responder {
	if let Some(ad) = &state.audio_data {
		if ad.a2ts.send(qint_proxy::audio::ResetMsg).await.is_err() {
			error!("Failed to reset audio pipeline");
			HttpResponse::InternalServerError()
		} else if ad.ts2a.send(qint_proxy::audio::ResetMsg).await.is_err() {
			error!("Failed to reset audio pipeline");
			HttpResponse::InternalServerError()
		} else {
			HttpResponse::Ok()
		}
	} else {
		HttpResponse::Ok()
	}
}

#[get("/graphiql")]
pub async fn graphiql() -> impl Responder {
	HttpResponse::Ok().content_type("text/html; charset=utf-8").body(graphiql_source("/db", None))
}

#[post("/db")]
pub(crate) async fn db_graphql(
	state: web::Data<Arc<QintState>>, req: web::Json<GraphQLRequest>,
) -> actix_web::Result<impl Responder> {
	let res = req.execute(&state.graphql_schema, &*state).await;
	let json_res = serde_json::to_string(&res)?;
	let mut resp = if res.is_ok() { HttpResponse::Ok() } else { HttpResponse::BadRequest() };
	Ok(resp.content_type("application/json").body(json_res))
}

/// Check the authentication token.
///
/// Returns an http response if this request is handled by an error or redirect.
/// If the result is `None`, the token is ok.
fn check_authentication(token: &str, req: &actix_web::dev::ServiceRequest) -> Option<HttpResponse> {
	#[derive(Deserialize)]
	pub struct TokenQuery {
		token: String,
	}

	if req.path() == "/" {
		if let Ok(Query(TokenQuery { token })) = Query::from_query(req.query_string()) {
			// Redirect to / and set cookie with token
			return Some(
				HttpResponse::SeeOther()
					.append_header((http::header::LOCATION, "/"))
					.cookie(cookie::Cookie::build("qint-auth", token).http_only(true).finish())
					.finish(),
			);
		}
	}

	// Check auth cookie
	if let Some(cookie) = req.cookie("qint-auth") {
		if cookie.value() == token {
			None
		} else {
			Some(HttpResponse::Forbidden().body(
				"Authentication token is wrong, please get a valid authentication token from the \
				 qint proxy",
			))
		}
	} else {
		Some(HttpResponse::Forbidden().body(
			"Authentication token is missing, please get a valid authentication token from the \
			 qint proxy",
		))
	}
}

/// Tests need a running TeamSpeak server on localhost. The default channel has to be channel 1,
/// this is used to access messages.
#[cfg(test)]
mod tests {
	use std::borrow::Cow;
	use std::future::Future;
	use std::time::Duration;

	use anyhow::{Result, bail, format_err};
	use base64::prelude::*;
	use futures::{SinkExt, StreamExt};
	use juniper::http::GraphQLRequest;
	use once_cell::sync::Lazy;
	use qint_proxy::messages::{ConnectOptions, JsMessageTarget, MessageF2P, MessageP2F};
	use qint_proxy::{ConnectionId, QintState};
	use rand::Rng;
	use serde::{Deserialize, Serialize};
	use tokio::time;
	use tracing::{debug, error, info};
	use tsclientlib::ClientId;
	use uuid::Uuid;

	use crate::Args;
	use crate::web::WebApp;
	use crate::websocket::{ConArgs, F2PMsg, P2FMsg, PassWsMsgArgs};

	static TRACING: Lazy<()> = Lazy::new(|| tracing_subscriber::fmt().with_test_writer().init());

	struct TestProxy {
		port: u16,
	}

	struct Connection {
		socket: actix_codec::Framed<awc::BoxedSocket, actix_http::ws::Codec>,
		id: ConnectionId,
		return_code: u32,
	}

	#[derive(Deserialize)]
	struct GraphQLResponse<T> {
		data: T,
	}

	#[derive(Deserialize)]
	struct ClientServerKey {
		/// Public key of the server.
		server: Vec<u8>,
		/// Uid of the own identity.
		client: Vec<u8>,
	}

	fn create_logger() { Lazy::force(&TRACING); }

	impl TestProxy {
		fn new() -> Self {
			let mut rng = rand::thread_rng();
			Self { port: rng.gen_range(1025..=65535) }
		}

		async fn create_connection(&self) -> Result<Connection> {
			let client = awc::Client::default();
			let id = ConnectionId(Uuid::new_v4());
			let url = format!("ws://127.0.0.1:{}/ws", self.port);
			info!(%url, "Connecting to proxy");
			let (_resp, socket) = client
				.ws(url)
				.connect()
				.await
				.map_err(|e| format_err!("Websocket client error: {:?}", e))?;
			let mut con = Connection { socket, return_code: 0, id };
			con.send_raw("create_ws", ConArgs { con: con.id }).await?;
			Ok(con)
		}

		async fn graphql<T>(&self, request: &GraphQLRequest) -> Result<T>
		where for<'a> T: Deserialize<'a> {
			let client = awc::Client::default();
			let url = format!("http://127.0.0.1:{}/db", self.port);
			debug!(body = %serde_json::to_string(&request).unwrap(), "GraphQL request");
			let mut resp = client
				.post(url)
				.send_json(request)
				.await
				.map_err(|_| format_err!("GraphQL failed"))?;
			if !resp.status().is_success() {
				let body = resp
					.body()
					.await
					.map_err(|e| format_err!("Failed to receive body: {:?}", e))?;
				bail!("GraphQL request failed: {}", String::from_utf8_lossy(body.as_ref()));
			}
			let resp: GraphQLResponse<T> =
				resp.json().await.map_err(|e| format_err!("Failed to decode json: {:?}", e))?;
			Ok(resp.data)
		}

		async fn get_client_server_key(&self) -> Result<ClientServerKey> {
			#[derive(Deserialize)]
			#[serde(rename_all = "camelCase")]
			struct Server {
				public_key: Vec<u8>,
			}
			#[derive(Deserialize)]
			struct Client {
				uid: Vec<u8>,
			}
			#[derive(Deserialize)]
			struct Identity {
				client: Client,
			}
			#[derive(Deserialize)]
			struct Bookmark {
				server: Server,
				identity: Identity,
			}
			#[derive(Deserialize)]
			#[serde(rename_all = "camelCase")]
			struct Query {
				most_recent_bookmark: Bookmark,
			}

			let resp: Query = self
				.graphql(&GraphQLRequest::new(
					"{
					mostRecentBookmark {
						server {
							publicKey
						}
						identity {
							client {
								uid
							}
						}
					}
				}"
					.into(),
					None,
					None,
				))
				.await?;
			Ok(ClientServerKey {
				client: resp.most_recent_bookmark.identity.client.uid,
				server: resp.most_recent_bookmark.server.public_key,
			})
		}

		/// Returns uid and name of the client and messages.
		async fn get_messages(
			&self, server: &[u8], type_s: &str, id: &str,
		) -> Result<Vec<(Vec<u8>, String, String)>> {
			#![allow(non_snake_case)]

			#[derive(Deserialize)]
			struct Client {
				uid: Vec<u8>,
				name: String,
			}
			#[derive(Deserialize)]
			struct Invoker {
				client: Client,
			}
			#[derive(Deserialize)]
			struct Message {
				invoker: Invoker,
				content: String,
			}
			#[derive(Deserialize)]
			struct Chat {
				messages: Vec<Message>,
			}
			#[derive(Deserialize)]
			struct Query {
				chat: Chat,
			}

			let vars = vec![("typ", type_s), ("id", id)];
			let vars = juniper::InputValue::Object({
				let mut vars: Vec<_> = vars
					.into_iter()
					.map(|(k, v)| {
						(
							juniper::parser::Spanning::unlocated(k.to_string()),
							juniper::parser::Spanning::unlocated(juniper::InputValue::scalar(v)),
						)
					})
					.collect();
				vars.push((
					juniper::parser::Spanning::unlocated("server".to_string()),
					juniper::parser::Spanning::unlocated(juniper::InputValue::list(
						server.iter().map(|b| juniper::InputValue::scalar(*b as i32)).collect(),
					)),
				));
				vars
			});
			let resp: Query = self
				.graphql(&GraphQLRequest::new(
					"query ($typ: GMessageTarget!, $server: [Int!]!, $id: ID!) {
					chat(typ: $typ, server: $server, id: $id) {
						messages {
							invoker {
								client {
									uid
									name
								}
							}
							content
						}
					}
				}"
					.into(),
					None,
					Some(vars),
				))
				.await?;
			Ok(resp
				.chat
				.messages
				.into_iter()
				.map(|m| (m.invoker.client.uid, m.invoker.client.name, m.content))
				.collect())
		}

		fn run(&self) -> impl Future<Output = Result<()>> {
			let port = self.port;
			async move {
				let dir = tempfile::Builder::new().prefix("qint-proxy").tempdir()?;
				info!(dir = %dir.path().display(), "Using config directory");
				let args = Args {
					listen_address: Some(format!("127.0.0.1:{}", port).parse().unwrap()),
					default_identity: None,
					config_path: Some(dir.path().join("config")),
					cache_path: Some(dir.path().join("cache")),
					plugin_path: None,
					no_audio: true,
					no_search: false,
					no_link_cache: false,
					no_open: true,
					verbosity: 1,
				};
				let app = WebApp::new(QintState::new(args.into())?);
				app.serve().await?;
				dir.close()?;
				Ok(())
			}
		}

		fn run_log_errors(&self) -> impl Future<Output = ()> {
			let fut = self.run();
			async move {
				if let Err(error) = fut.await {
					error!(%error, "Proxy encountered an error");
				}
			}
		}
	}

	impl Connection {
		async fn connect(&mut self) -> Result<ClientId> {
			self.send(MessageF2P::Connect(ConnectOptions {
				address: "localhost".to_string(),
				name: "Test".to_string(),
				..Default::default()
			}))
			.await?;
			loop {
				let msg = self.recv().await?;
				if let Some(MessageP2F::Connected { own_client, .. }) = msg {
					return Ok(ClientId(own_client.parse().unwrap()));
				} else if let Some(MessageP2F::Error(e)) = msg {
					bail!("Got proxy error: {}", e);
				}
			}
		}

		async fn send_raw<M: Serialize>(&mut self, cmd: &str, msg: M) -> Result<()> {
			let msg_str = serde_json::to_string(&msg).unwrap();
			info!(%cmd, msg = %msg_str, "Sending message to proxy");
			let return_code = self.return_code;
			self.return_code += 1;
			let msg = F2PMsg {
				cmd: cmd.into(),
				return_code: return_code.to_string(),
				args: serde_json::to_value(msg)?.into(),
			};
			self.socket
				.send(actix_ws::Message::Text(serde_json::to_string(&msg)?.into()))
				.await
				.map_err(|e| format_err!("Websocket client protocol error: {:?}", e))?;
			Ok(())
		}

		async fn send(&mut self, msg: MessageF2P) -> Result<()> {
			self.send_raw("pass_ws_msg", PassWsMsgArgs { con: self.id, msg }).await
		}

		async fn recv(&mut self) -> Result<Option<MessageP2F>> {
			match self.socket.next().await {
				Some(Ok(actix_http::ws::Frame::Text(msg))) => {
					let msg = std::str::from_utf8(&msg)?;
					info!(%msg, "Received message from proxy");
					let msg: P2FMsg = serde_json::from_str(msg)?;
					Ok(msg.msg.map(Cow::into_owned))
				}
				f => bail!("Websocket client received unexpected packet: {:?}", f),
			}
		}
	}

	/// Check that connecting to a server adds this server to the recent connections and updates
	/// it when reconnecting.
	#[actix_rt::test]
	async fn test_save_server() -> Result<()> {
		create_logger();
		let proxy = TestProxy::new();
		actix::spawn(proxy.run_log_errors());
		// Wait for server to come up
		time::sleep(Duration::from_millis(100)).await;
		let mut con = proxy.create_connection().await?;
		con.connect().await?;
		// Wait for saving the connection in the database
		time::sleep(Duration::from_millis(100)).await;
		drop(con);

		#[derive(Deserialize)]
		#[serde(rename_all = "camelCase")]
		struct ServerServer {
			#[allow(dead_code)]
			public_key: Vec<u8>,
		}
		#[derive(Deserialize)]
		struct ServerBookmark {
			#[allow(dead_code)]
			server: ServerServer,
		}
		#[derive(Deserialize)]
		struct ServerResponse {
			bookmarks: Vec<ServerBookmark>,
		}

		// Check for the server in the database
		let response: ServerResponse = proxy
			.graphql(&GraphQLRequest::new(
				"{
				bookmarks {
					server {
						publicKey
					}
				}
			}"
				.into(),
				None,
				None,
			))
			.await?;
		assert_eq!(response.bookmarks.len(), 1, "Recent connection not saved in the database");
		Ok(())
	}

	/// Check that getting or sending a message from a client saves the other client and the
	/// message.
	#[actix_rt::test]
	async fn test_save_client() -> Result<()> {
		create_logger();
		let proxy0 = TestProxy::new();
		actix::spawn(proxy0.run_log_errors());
		let proxy1 = TestProxy::new();
		actix::spawn(proxy1.run_log_errors());
		// Wait for server to come up
		time::sleep(Duration::from_millis(100)).await;
		let mut con0 = proxy0.create_connection().await?;
		con0.connect().await?;
		let mut con1 = proxy1.create_connection().await?;
		let con1_id = con1.connect().await?;

		// con0 sends a message to con1
		let msg = "Hello 1";
		con0.send(MessageF2P::SendMessage {
			target: JsMessageTarget::Client(con1_id),
			message: msg.to_string(),
			return_code: None,
		})
		.await?;

		// Wait for saving the message in the database
		time::sleep(Duration::from_millis(100)).await;
		drop(con0);
		drop(con1);

		let key0 = proxy0.get_client_server_key().await?;
		let key1 = proxy1.get_client_server_key().await?;

		// Check for the message in the database of con0
		let msgs = proxy0
			.get_messages(&key0.server, "CLIENT", &BASE64_STANDARD.encode(&key1.client))
			.await?;
		assert_eq!(msgs.len(), 1, "Message not saved in the database");
		assert_eq!(msgs[0].0, key0.client, "Sender uid is wrong");
		assert_eq!(msgs[0].2, msg, "Message is wrong");
		assert!(msgs[0].1.starts_with("Test"), "Client name has to start with 'Test'");

		// Check for the message in the database of con1
		let msgs = proxy1
			.get_messages(&key0.server, "CLIENT", &BASE64_STANDARD.encode(&key0.client))
			.await?;
		assert_eq!(msgs.len(), 1, "Message not saved in the database");
		assert_eq!(msgs[0].0, key0.client, "Sender uid is wrong");
		assert_eq!(msgs[0].2, msg, "Message is wrong");
		assert!(msgs[0].1.starts_with("Test"), "Client name has to start with 'Test'");
		Ok(())
	}
}
