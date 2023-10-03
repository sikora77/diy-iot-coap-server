use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use coap_lite::link_format::LINK_ATTR_RESOURCE_TYPE;
use coap_lite::{ContentFormat, MessageClass, ResponseType};
use tokio::sync::{oneshot, Mutex};
use tokio::time;

use coap_server::app::ObservableResource;
use coap_server::app::{AppBuilder, CoapError, Observers, ObserversHolder, Request, Response};
use coap_server::FatalServerError;
use coap_server::{app, CoapServer, UdpTransport};
use env_logger;
#[macro_use]
extern crate serde_derive;

#[tokio::main]
async fn main() -> Result<(), FatalServerError> {
	env_logger::init();
	let server = CoapServer::bind(UdpTransport::new("sikora-laptop.local:5683")).await?;
	server.serve(build_app()).await
}

fn build_app() -> AppBuilder<SocketAddr> {
	let light_state = LightsState::default();
	let state_for_get = light_state.clone();
	let state_for_put = light_state.clone();
	let state_for_create_put = light_state.clone();
	let state_for_is_online = light_state.clone();
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
		)
}

pub mod states;

use crate::states::LightState;

#[derive(Default, Clone)]
struct LightsState {
	data: Arc<Mutex<HashMap<String, LightState>>>,
	observers: ObserversHolder,
}

#[async_trait]
impl ObservableResource for LightsState {
	async fn on_active(&self, observers: Observers) -> Observers {
		let relative_path = observers.relative_path();
		//let device_id = relative_path.split('/').last().unwrap().to_string();
		println!("Observe active for path: {relative_path}...");
		let attached = self.observers.attach(observers).await;
		let (tx, mut rx) = oneshot::channel();
		let observers = self.observers.clone();
		//let mut data = self.data.clone();
		tokio::spawn(async move {
			let mut interval = time::interval(Duration::from_secs(5));
			loop {
				tokio::select! {
					_ = &mut rx => {
					   return
					}
					_ = interval.tick() => {
						observers.notify_change().await;
					}
				}
			}
		});
		attached.stay_active().await;
		tx.send(()).unwrap();
		println!("Observe no longer active for path: {relative_path}!");
		attached.detach().await
	}
}

async fn handle_get_lights(
	request: Request<SocketAddr>,
	state: LightsState,
) -> Result<Response, CoapError> {
	println!("registered request");
	let mut response = request.new_response();
	let mut full_path = "".to_string();
	for path in request.unmatched_path.into_iter() {
		full_path += &path;
	}
	// println!(
	// 	"{}",
	// 	state
	// 		.observers
	// 		.get_count(format!("lights/{}", &full_path).as_str())
	// 		.await
	// );
	match state.data.lock().await.get(full_path.as_str()) {
		Some(x) => {
			response
				.message
				.set_content_format(ContentFormat::TextPlain);
			response.message.payload = serde_json::to_string(x).unwrap().as_bytes().to_vec();
			Ok(response)
		}
		None => {
			response.set_status(ResponseType::NotFound);
			response.message.header.code = MessageClass::Response(ResponseType::NotFound);
			Ok(response)
		}
	}
	// println!("{full_path}");
	// full_path = format!("lights/{full_path}");
}

async fn handle_is_online(
	request: Request<SocketAddr>,
	state: LightsState,
) -> Result<Response, CoapError> {
	let mut response = request.new_response();
	let mut full_path = "".to_string();
	for path in request.unmatched_path.into_iter() {
		full_path += &path;
	}
	let relative_path = format!("lights/{}", &full_path);
	println!("{}", relative_path);
	// Wont work with stock library
	let observer_count = 2; //state.observers.get_count(relative_path.as_str()).await;
						// if observer_count > 0 {
						// 	println!("hii");
						// }
	match state.data.lock().await.get(full_path.as_str()) {
		Some(x) => {
			response
				.message
				.set_content_format(ContentFormat::TextPlain);
			response.message.payload = serde_json::to_string(&observer_count)
				.unwrap()
				.as_bytes()
				.to_vec();
			Ok(response)
		}
		None => {
			response.set_status(ResponseType::NotFound);
			response.message.header.code = MessageClass::Response(ResponseType::NotFound);
			Ok(response)
		}
	}
	// println!("{full_path}");
	// full_path = format!("lights/{full_path}");
}

// Changing the light state
async fn handle_put_lights(
	request: Request<SocketAddr>,
	state: LightsState,
) -> Result<Response, CoapError> {
	let mut response = request.new_response();
	let mut full_path = "".to_string();
	for path in request.unmatched_path.clone().into_iter() {
		full_path += &path;
	}
	println!(
		"{}",
		String::from_utf8(request.original.message.payload.clone()).unwrap()
	);
	let new_state: LightState =
		serde_json::from_str(&String::from_utf8(request.original.message.payload).unwrap())
			.unwrap();
	let device_id = request.unmatched_path.last().unwrap();
	let mut data = state.data.lock().await;
	println!("handling put");
	let data_clone = data.clone();
	let current_state = data_clone.get(device_id);
	if current_state.is_none() {
		println!("device not found");
		return Err(CoapError::not_found());
	}
	data.insert(device_id.clone(), new_state);
	println!("{device_id}");
	full_path = format!("lights/{full_path}");
	println!("{}", full_path);
	response
		.message
		.set_content_format(ContentFormat::TextPlain);
	state.observers.notify_change_for_path(&full_path).await;
	Ok(response)
}

// Creating the device
async fn handle_device_create_put(
	request: Request<SocketAddr>,
	state: LightsState,
) -> Result<Response, CoapError> {
	println!("Creating device");
	let mut response = request.new_response();
	let mut full_path = "".to_string();
	let payload = String::from_utf8(request.original.message.payload).unwrap();
	println!("{}", payload.as_ref() as &str);
	for path in request.unmatched_path.clone().into_iter() {
		full_path += &path;
	}
	let mut data = state.data.lock().await;
	let len = data.len() + 1;
	data.insert(
		payload,
		LightState {
			is_on: true,
			color: 255 * 255 * 255,
			brightness: 255,
		},
	);
	response
		.message
		.set_content_format(ContentFormat::TextPlain);
	response.message.payload = len.to_string().as_bytes().to_vec();
	Ok(response)
}
