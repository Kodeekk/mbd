use anyhow::{anyhow, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::StreamExt;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

const BTN_CHAR: &str = "0000000c-1337-1dea-feed-c0ffee70c0de";
const DEBOUNCE_MS: u64 = 1500;

pub struct Daemon {
    command: String,
    verbose: bool,
    mac: Option<String>,
    script_mode: bool,
}

impl Daemon {
    pub fn new(command: String, verbose: bool, mac: Option<String>, script_mode: bool) -> Self {
        Self { command, verbose, mac, script_mode }
    }

    pub async fn run(&self) -> Result<()> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No BLE adapter found"))?;

        println!("mbd: using adapter {}", adapter.adapter_info().await?);

        loop {
            match self.connect_and_listen(&adapter).await {
                Ok(()) => println!("mbd: connection ended, reconnecting in 5s..."),
                Err(e) => eprintln!("mbd: error: {e:#}, reconnecting in 5s..."),
            }
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn connect_and_listen(&self, adapter: &Adapter) -> Result<()> {
        let peripheral = self.find_device(adapter).await?;

        if !peripheral.is_connected().await? {
            println!("mbd: connecting to {}...", peripheral.address());
            peripheral.connect().await?;
        }
        println!("mbd: connected ({})", peripheral.address());

        peripheral.discover_services().await?;

        let btn_uuid = Uuid::parse_str(BTN_CHAR)?;
        let btn_char = peripheral
            .characteristics()
            .iter()
            .find(|c| c.uuid == btn_uuid)
            .ok_or_else(|| anyhow!("M-Button characteristic not found"))?
            .clone();

        peripheral.subscribe(&btn_char).await?;
        println!("mbd: listening (Ctrl+C to stop)");

        let mut last = Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1);
        let mut cmd_n = 0u64;
        let mut evt_n = 0u64;

        let mut stream = peripheral.notifications().await?;
        while let Some(notif) = stream.next().await {
            if notif.uuid != btn_char.uuid {
                continue;
            }
            evt_n += 1;

            if self.verbose {
                let hex: String = notif.value.iter().map(|b| format!("{b:02x}")).collect();
                println!("  [{evt_n}] {hex}");
            }

            if notif.value == vec![0x00, 0x00, 0x09] {
                let now = Instant::now();
                if now.duration_since(last).as_millis() < DEBOUNCE_MS as u128 {
                    continue;
                }
                last = now;
                cmd_n += 1;
                println!("[M-Button] #{cmd_n}");

                if !self.command.is_empty() {
                    let result = if self.script_mode {
                        std::process::Command::new(&self.command).spawn()
                    } else {
                        std::process::Command::new("sh")
                            .arg("-c")
                            .arg(&self.command)
                            .spawn()
                    };
                    if let Err(e) = result {
                        eprintln!("  failed to spawn command: {e}");
                    }
                }
            }

            if !peripheral.is_connected().await? {
                println!("mbd: disconnected");
                break;
            }
        }

        Ok(())
    }

    async fn find_device(&self, adapter: &Adapter) -> Result<Peripheral> {
        adapter.start_scan(ScanFilter::default()).await?;

        if let Some(ref mac) = self.mac {
            let target = mac.to_uppercase();
            for _ in 0..100 {
                for p in adapter.peripherals().await?.iter() {
                    if p.address().to_string().to_uppercase() == target {
                        adapter.stop_scan().await?;
                        return Ok(p.clone());
                    }
                }
                sleep(Duration::from_millis(100)).await;
            }
        } else {
            for _ in 0..100 {
                for p in adapter.peripherals().await?.iter() {
                    if let Some(props) = p.properties().await? {
                        if let Some(name) = props.local_name {
                            let up = name.to_uppercase();
                            if up.contains("MAJOR V")
                                || up.contains("MAJOR 5")
                                || up.starts_with("MAJOR")
                            {
                                println!("mbd: found {name} @ {}", p.address());
                                adapter.stop_scan().await?;
                                return Ok(p.clone());
                            }
                        }
                    }
                }
                sleep(Duration::from_millis(100)).await;
            }
        }

        adapter.stop_scan().await?;
        Err(anyhow!("Device not found after scanning"))
    }
}
