use std::net::SocketAddr;

use crate::lights::{handle_device_remove_put, LightsState};
use coap_lite::link_format::LINK_ATTR_RESOURCE_TYPE;
use coap_server::app::AppBuilder;
use coap_server::FatalServerError;
use coap_server::{app, CoapServer};
use coap_server_tokio::transport::udp::UdpTransport;
use dotenv::dotenv;
use lights::{handle_device_create_put, handle_get_lights, handle_is_online, handle_put_lights};
#[macro_use]
extern crate serde_derive;

pub mod lights;
pub mod states;

#[tokio::main]
async fn main() -> Result<(), FatalServerError> {
	env_logger::init();
	dotenv().ok();
	let server_addr = dotenv::var("ADDR").expect("set ADDR");
	let server_port = dotenv::var("PORT").expect("set PORT");
	let server = CoapServer::bind(UdpTransport::new(format!(
		"{}:{}",
		server_addr, server_port
	)))
	.await?;
	server.serve(build_app()).await
}

fn build_app() -> AppBuilder<SocketAddr> {
	let light_state = LightsState::default();
	let state_for_get = light_state.clone();
	let state_for_put = light_state.clone();
	let state_for_create_put = light_state.clone();
	let state_for_is_online = light_state.clone();
	let state_for_remove = light_state.clone();
	app::new()
		.resource(
			app::resource("/lights")
				.link_attr(LINK_ATTR_RESOURCE_TYPE, "lights")
				.observable(light_state)
				.get(move |req| handle_get_lights(req, state_for_get.clone()))
				.put(move |req| handle_put_lights(req, state_for_put.clone())),
		)
		.resource(
			app::resource("/lights/create")
				.put(move |req| handle_device_create_put(req, state_for_create_put.clone())),
		)
		.resource(
			app::resource("/lights/is_online")
				.get(move |req| handle_is_online(req, state_for_is_online.clone())),
		).resource(
			app::resource("/lights/remove")
				.get(move |req| handle_device_remove_put(req, state_for_remove.clone())),
		)
}
