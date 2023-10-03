use std::net::SocketAddr;

use coap_server::app::{CoapError, Request, Response};
use coap_server::{app, CoapServer, FatalServerError, UdpTransport};

#[tokio::main]
async fn main() -> Result<(), FatalServerError> {
	let server = CoapServer::bind(UdpTransport::new("0.0.0.0:5683")).await?;
	server
		.serve(app::new().resource(app::resource("/").default_handler(handle_all)))
		.await
}

async fn handle_all(request: Request<SocketAddr>) -> Result<Response, CoapError> {
	let method = *request.original.get_method();
	let path = request.original.get_path();
	let peer = request.original.source.unwrap();
	let mut response = request.new_response();
	println!("{}", format!("Received from {peer:?}: {method:?} /{path}"));
	response.message.payload = format!("Received from {peer:?}: {method:?} /{path}").into_bytes();

	Ok(response)
}
