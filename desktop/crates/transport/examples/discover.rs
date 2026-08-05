//! Development probe: browses `_tandem._tcp` for a few seconds and prints the
//! first phone it finds, so mDNS reachability can be checked without pairing.

use std::time::Duration;

#[tokio::main]
async fn main() {
    match tandem_transport::discovery::find_any_phone(Duration::from_secs(10)).await {
        Ok(Some(phone)) => println!(
            "found {} ({}) at {}:{}",
            phone.display_name, phone.device_id, phone.host, phone.port
        ),
        Ok(None) => println!("no Tandem phone advertising on this network"),
        Err(error) => println!("discovery failed: {error}"),
    }
}
