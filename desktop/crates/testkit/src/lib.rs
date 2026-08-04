//! tandem_testkit: deterministic fakes for every desktop I/O seam (transport,
//! Bluetooth, audio, phone peer, HFP AG) plus shared fixtures, backing the test
//! pyramid in docs/15.

pub mod fake_ag;
pub mod fake_audio_backend;
pub mod fake_bluetooth_backend;
pub mod fake_phone;
pub mod fake_transport;
pub mod fixtures;

pub use fake_ag::FakeAudioGateway;
pub use fake_audio_backend::FakeAudioBackend;
pub use fake_bluetooth_backend::FakeBluetoothBackend;
pub use fake_phone::FakePhone;
pub use fake_transport::FakeTransport;
