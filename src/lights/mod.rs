use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use crate::states::LightState;
use async_trait::async_trait;
use coap_lite::{ContentFormat, MessageClass, ResponseType};
use coap_server::app::{
	CoapError, ObservableResource, Observers, ObserversHolder, Request, Response,
};
use serde_json::json;
use tokio::{
	sync::{oneshot, Mutex},
	time,
};

#[derive(Default, Clone)]
pub struct LightsState {
	data: Arc<Mutex<HashMap<String, LightState>>>,
	observers: ObserversHolder,
}

#[async_trait]
impl ObservableResource for LightsState {
	async fn on_active(&self, observers: Observers) -> Observers {
		let relative_path = observers.relative_path();
		println!("Observe active for path: {relative_path}...");
		let attached = self.observers.attach(observers).await;
		let (tx, mut rx) = oneshot::channel();
		let observers = self.observers.clone();
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

pub async fn handle_get_lights(
	request: Request<SocketAddr>,
	state: LightsState,
) -> Result<Response, CoapError> {
	// println!("registered request");
	let mut response = request.new_response();
	let light_id = &request.unmatched_path[0];

	// let mut full_path = "".to_string();
	println!("{}", light_id);
	// for path in request.unmatched_path.into_iter() {
	// 	full_path += &path;
	// }
	match state.data.lock().await.get(light_id) {
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
}

pub async fn handle_is_online(
	request: Request<SocketAddr>,
	state: LightsState,
) -> Result<Response, CoapError> {
	let mut response = request.new_response();
	let light_id = &request.unmatched_path[0];
	// let mut full_path = "".to_string();
	// for path in request.unmatched_path.into_iter() {
	// 	full_path += &path;
	// }
	let relative_path = format!("lights/{}", light_id);
	println!("{}", relative_path);
	// Wont work with stock library
	let observer_count = state.observers.get_count(&relative_path).await;
	println!("{}", observer_count);
	match state.data.lock().await.get(light_id) {
		Some(_) => {
			response
				.message
				.set_content_format(ContentFormat::TextPlain);
			response.message.payload = serde_json::to_string(&json!({"isOnline":observer_count>0}))
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
}

// Changing the light state
pub async fn handle_put_lights(
	request: Request<SocketAddr>,
	state: LightsState,
) -> Result<Response, CoapError> {
	let mut response = request.new_response();
	let light_id = &request.unmatched_path[0];
	// let mut full_path = "".to_string();
	// for path in request.unmatched_path.clone().into_iter() {
	// 	full_path += &path;
	// }
	println!(
		"{}",
		String::from_utf8(request.original.message.payload.clone()).unwrap()
	);
	let is_removed: bool = match state.data.lock().await.get(light_id) {
		Some(x) => x.removed,
		None => false,
	};

	let full_path = format!("lights/{}", light_id);

	let new_state: LightState =
		serde_json::from_str(&String::from_utf8(request.original.message.payload).unwrap())
			.unwrap();
	let device_id = request.unmatched_path.last().unwrap();
	if is_removed {
		println!("{}", full_path);
		response
			.message
			.set_content_format(ContentFormat::TextPlain);
		state.observers.notify_change_for_path(&full_path).await;
		return Ok(response);
	}
	let mut data = state.data.lock().await;
	println!("Changing state of device {}", device_id);
	let data_clone = data.clone();
	let current_state = data_clone.get(device_id);
	if current_state.is_none() {
		println!("device not found");
		return Err(CoapError::not_found());
	}
	data.insert(device_id.clone(), new_state);
	println!("{device_id}");
	println!("{}", full_path);
	response
		.message
		.set_content_format(ContentFormat::TextPlain);
	state.observers.notify_change_for_path(&full_path).await;
	Ok(response)
}

// Creating the device
pub async fn handle_device_create_put(
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
			removed: false,
		},
	);
	response
		.message
		.set_content_format(ContentFormat::TextPlain);
	response.message.payload = len.to_string().as_bytes().to_vec();
	Ok(response)
}
pub async fn handle_device_remove_put(
	request: Request<SocketAddr>,
	state: LightsState,
) -> Result<Response, CoapError> {
	let mut response = request.new_response();
	let mut full_path = "".to_string();
	let payload = String::from_utf8(request.original.message.payload).unwrap();
	println!("{}", payload.as_ref() as &str);
	for path in request.unmatched_path.clone().into_iter() {
		full_path += &path;
	}
	let mut data = state.data.lock().await;
	data.insert(
		payload.clone(),
		LightState {
			is_on: true,
			color: 255 * 255 * 255,
			brightness: 255,
			removed: true,
		},
	);
	let len = data.len() + 1;
	println!("Removing device: {}", payload);
	response
		.message
		.set_content_format(ContentFormat::TextPlain);
	response.message.payload = len.to_string().as_bytes().to_vec();
	Ok(response)
}
