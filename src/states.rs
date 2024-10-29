#[derive(Serialize, Deserialize, Clone)]
pub struct LightState {
	pub is_on: bool,
	pub brightness: i32,
	pub color: i32,
	pub removed: bool,
}
