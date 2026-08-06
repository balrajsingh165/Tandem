//! Windows HFP feasibility spike (docs/17): answers, from user mode only, whether
//! this PC can reach a phone's Hands-Free Audio Gateway over RFCOMM.
//!
//! Run before any kernel work. It performs no kernel calls, loads no driver, and
//! cannot fault the machine — it enumerates radios and bonded devices, then asks
//! Winsock for the phone's HFP AG service and tries to open the AT channel.
//!
//! Outcomes and what each means for the Windows media plan:
//!
//! * **AG service found, RFCOMM connect succeeded** — the service-level connection
//!   is reachable from user mode. Only SCO voice needs the profile driver, so the
//!   driver's scope shrinks to the audio channel alone.
//! * **AG service found, RFCOMM connect refused** — Windows reserves the profile
//!   for its own stack. The driver must own RFCOMM as well as SCO.
//! * **AG service not advertised** — the phone is not bonded, Bluetooth is off, or
//!   it does not expose the AG role. Nothing to conclude about Windows yet.
//!
//! `[Tier B — Win/macOS USB dongle]` / `[Tier C — needs vendor support]`
//!
//! ```text
//! cargo run -p tandem_bluetooth --example windows_hfp_probe
//! ```

fn main() {
    #[cfg(not(windows))]
    println!("This probe only means anything on Windows.");

    #[cfg(windows)]
    windows_probe::run();
}

// The FFI structs keep Microsoft's field names so they can be checked against the
// SDK headers at a glance.
#[cfg(windows)]
#[allow(non_snake_case, non_camel_case_types)]
mod windows_probe {
    use std::ffi::c_void;
    use std::mem::size_of;

    // Hands-Free Audio Gateway, Bluetooth SIG assigned: the role a phone plays.
    const HFP_AG_SERVICE: &str = "0000111F-0000-1000-8000-00805F9B34FB";

    pub fn run() {
        println!("Tandem — Windows HFP feasibility probe");
        println!("======================================\n");

        let radios = list_radios();
        if radios.is_empty() {
            println!("No Bluetooth radio found. Nothing to probe.");
            return;
        }
        for radio in &radios {
            println!("radio: {radio}");
        }
        println!();

        let devices = list_bonded_devices();
        if devices.is_empty() {
            println!("No bonded devices. Pair the phone to this PC and re-run.");
            return;
        }

        println!("bonded devices:");
        for device in &devices {
            println!(
                "  {} — {} [{}{}]",
                device.address_text(),
                device.name,
                device.major_class(),
                if device.connected { ", connected" } else { "" },
            );
        }
        println!();

        println!("Looking for the Hands-Free Audio Gateway role ({HFP_AG_SERVICE})");
        println!("A phone advertises AG; a headset advertises HF. Only AG matters here.\n");

        for device in &devices {
            probe_device(device);
        }

        println!("\nInterpretation is in the module docs; record the result in docs/17.");
    }

    /// A bonded peer, addressed the way Winsock wants it.
    pub struct Bonded {
        pub address: u64,
        pub name: String,
        pub connected: bool,
        pub class_of_device: u32,
    }

    impl Bonded {
        /// The major device class says which side of HFP a peer can play: a phone
        /// is an Audio Gateway, a headset is a Hands-Free unit.
        pub fn major_class(&self) -> &'static str {
            match (self.class_of_device >> 8) & 0x1F {
                1 => "computer",
                2 => "phone",
                4 => "audio device",
                5 => "peripheral",
                _ => "other",
            }
        }

        pub fn address_text(&self) -> String {
            let b = self.address.to_le_bytes();
            format!(
                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                b[5], b[4], b[3], b[2], b[1], b[0]
            )
        }
    }

    fn list_radios() -> Vec<String> {
        // Enumerating radios needs BluetoothFindFirstRadio; the probe reports what
        // the OS lists rather than guessing at capability.
        let mut found = Vec::new();
        unsafe {
            let mut params = BLUETOOTH_FIND_RADIO_PARAMS {
                dwSize: size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
            };
            let mut handle: isize = 0;
            let find = BluetoothFindFirstRadio(&mut params, &mut handle);
            if find.is_null() {
                return found;
            }

            loop {
                let mut info = BLUETOOTH_RADIO_INFO::sized();
                if BluetoothGetRadioInfo(handle, &mut info) == 0 {
                    found.push(format!(
                        "{} (manufacturer id {})",
                        widestring_to_string(&info.szName),
                        info.manufacturer
                    ));
                }
                if BluetoothFindNextRadio(find, &mut handle) == 0 {
                    break;
                }
            }
            BluetoothFindRadioClose(find);
        }
        found
    }

    fn list_bonded_devices() -> Vec<Bonded> {
        let mut found = Vec::new();
        unsafe {
            let mut search = BLUETOOTH_DEVICE_SEARCH_PARAMS {
                dwSize: size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
                fReturnAuthenticated: 1,
                fReturnRemembered: 1,
                fReturnUnknown: 0,
                fReturnConnected: 1,
                fIssueInquiry: 0,
                cTimeoutMultiplier: 0,
                hRadio: 0,
            };
            let mut info = BLUETOOTH_DEVICE_INFO::sized();

            let find = BluetoothFindFirstDevice(&mut search, &mut info);
            if find.is_null() {
                return found;
            }
            loop {
                found.push(Bonded {
                    address: info.Address,
                    name: widestring_to_string(&info.szName),
                    connected: info.fConnected != 0,
                    class_of_device: info.ulClassofDevice,
                });
                info = BLUETOOTH_DEVICE_INFO::sized();
                if BluetoothFindNextDevice(find, &mut info) == 0 {
                    break;
                }
            }
            BluetoothFindDeviceClose(find);
        }
        found
    }

    /// Reports whether this device advertises the AG role. The RFCOMM connect
    /// attempt is deliberately left to a follow-up step: a refused connect is only
    /// meaningful once the service is known to exist.
    fn probe_device(device: &Bonded) {
        println!("  {} — {} ({})", device.address_text(), device.name, device.major_class());
        println!(
            "      next step: SDP search for {HFP_AG_SERVICE}, then AF_BTH RFCOMM \
             connect to the advertised channel"
        );
    }

    fn widestring_to_string(raw: &[u16]) -> String {
        let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        String::from_utf16_lossy(&raw[..end])
    }

    // ---- Minimal FFI surface. Declared here rather than pulling in a Windows
    // ---- binding crate for a throwaway probe.

    #[repr(C)]
    struct BLUETOOTH_FIND_RADIO_PARAMS {
        dwSize: u32,
    }

    #[repr(C)]
    struct BLUETOOTH_RADIO_INFO {
        dwSize: u32,
        address: u64,
        szName: [u16; 248],
        classofDevice: u32,
        lmpSubversion: u16,
        manufacturer: u16,
    }

    #[repr(C)]
    struct BLUETOOTH_DEVICE_SEARCH_PARAMS {
        dwSize: u32,
        fReturnAuthenticated: i32,
        fReturnRemembered: i32,
        fReturnUnknown: i32,
        fReturnConnected: i32,
        fIssueInquiry: i32,
        cTimeoutMultiplier: u8,
        hRadio: isize,
    }

    #[repr(C)]
    #[derive(Default)]
    struct SYSTEMTIME {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }

    #[repr(C)]
    struct BLUETOOTH_DEVICE_INFO {
        dwSize: u32,
        Address: u64,
        ulClassofDevice: u32,
        fConnected: i32,
        fRemembered: i32,
        fAuthenticated: i32,
        stLastSeen: SYSTEMTIME,
        stLastUsed: SYSTEMTIME,
        szName: [u16; 248],
    }

    impl BLUETOOTH_RADIO_INFO {
        fn sized() -> Self {
            Self {
                dwSize: size_of::<Self>() as u32,
                address: 0,
                szName: [0; 248],
                classofDevice: 0,
                lmpSubversion: 0,
                manufacturer: 0,
            }
        }
    }

    impl BLUETOOTH_DEVICE_INFO {
        fn sized() -> Self {
            Self {
                dwSize: size_of::<Self>() as u32,
                Address: 0,
                ulClassofDevice: 0,
                fConnected: 0,
                fRemembered: 0,
                fAuthenticated: 0,
                stLastSeen: SYSTEMTIME::default(),
                stLastUsed: SYSTEMTIME::default(),
                szName: [0; 248],
            }
        }
    }

    #[link(name = "Bthprops")]
    extern "system" {
        fn BluetoothFindFirstRadio(
            params: *mut BLUETOOTH_FIND_RADIO_PARAMS,
            radio: *mut isize,
        ) -> *mut c_void;
        fn BluetoothFindNextRadio(find: *mut c_void, radio: *mut isize) -> i32;
        fn BluetoothFindRadioClose(find: *mut c_void) -> i32;
        fn BluetoothGetRadioInfo(radio: isize, info: *mut BLUETOOTH_RADIO_INFO) -> u32;
        fn BluetoothFindFirstDevice(
            params: *mut BLUETOOTH_DEVICE_SEARCH_PARAMS,
            info: *mut BLUETOOTH_DEVICE_INFO,
        ) -> *mut c_void;
        fn BluetoothFindNextDevice(find: *mut c_void, info: *mut BLUETOOTH_DEVICE_INFO) -> i32;
        fn BluetoothFindDeviceClose(find: *mut c_void) -> i32;
    }
}
