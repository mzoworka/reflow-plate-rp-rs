use bincode::{Encode, Decode};
use embassy_rp::flash;

use crate::SyncStateChannelReceiver;
use crate::tools::BINCODE_CONFIG;

const FLASH_MAGIC: u8 = 0xB5;
const FLASH_VERSION: u8 = 0x02;
const FLASH_SIZE: usize = 2048*1024;
const STORAGE_OFFSET: u32 = (2048*1024) - 4096;
const STORAGE_SIZE: u32 = 4096;

pub(crate) enum SyncStorageStateEnum {
    WritePid((bool, f32, f32, f32)),
}

#[derive(Debug, Encode, Decode, Clone)]
pub(crate) struct Storage {
    magic: u8,
    version: u8,
    pub pid_p: f32,
    pub pid_i: f32,
    pub pid_d: f32,
    pub pid: bool,
}

impl Default for Storage {
    fn default() -> Self {
        Self { 
            magic: FLASH_MAGIC, 
            version: FLASH_VERSION, 
            pid_p: 0.0, 
            pid_i: 0.0, 
            pid_d: 0.0, 
            pid: false,
        }
    }
}

pub(crate) fn flash_read(flash: &mut flash::Flash<'_, embassy_rp::peripherals::FLASH, flash::Blocking, FLASH_SIZE>) -> Storage {
    let mut buf = [0; STORAGE_SIZE as usize];
    if flash.blocking_read(STORAGE_OFFSET, &mut buf).is_ok() {
        let mut storage: Storage = bincode::decode_from_slice(&buf, BINCODE_CONFIG).unwrap_or_default().0;
        
        //sanity checks
        if storage.magic != FLASH_MAGIC {
            storage = Storage::default();
        }
        if storage.version != FLASH_VERSION {
            storage = Storage::default();
        }
        if storage.pid_p.is_nan() {
            storage.pid_p = 0.0;
        }
        if storage.pid_i.is_nan() {
            storage.pid_i = 0.0;
        }
        if storage.pid_d.is_nan() {
            storage.pid_d = 0.0;
        }

        storage
    } else {
        Storage::default()
    }
}

pub(crate) async fn flash_task(startup_storage: &'_ Storage, flash: &'_ mut flash::Flash<'_, embassy_rp::peripherals::FLASH, flash::Blocking, FLASH_SIZE>, rx: SyncStateChannelReceiver<'_, SyncStorageStateEnum>) -> ! {
    let mut storage = startup_storage.clone();
    loop {
        let query = rx.receive().await;
        match query {
            SyncStorageStateEnum::WritePid(x) => {
                let mut buf = [0; STORAGE_SIZE as usize];

                storage.pid_p = x.1;
                storage.pid_i = x.2;
                storage.pid_d = x.3;
                storage.pid = x.0;

                bincode::encode_into_slice(&storage, &mut buf, BINCODE_CONFIG).expect("flashtask enc fail");
                
                flash.blocking_erase(STORAGE_OFFSET, STORAGE_OFFSET+STORAGE_SIZE).expect("flashtask erase fail");
                flash.blocking_write(STORAGE_OFFSET, &buf).expect("flashtask write fail");
            },
        }
    }
}
