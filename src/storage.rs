use bincode::{Decode, Encode};
use embassy_rp::flash;

use crate::{
    channels,
    tools::{SyncStateChannelReceiver, BINCODE_CONFIG},
};

const FLASH_MAGIC: u8 = 0xB5;
const FLASH_VERSION: u8 = 0x03;
const FLASH_SIZE: usize = 2048 * 1024;
const STORAGE_OFFSET: u32 = (2048 * 1024) - 4096;
const STORAGE_SIZE: u32 = 4096;

const MAX_WAIT_TIME_DEFAULT: f32 = 10.0;
const EXTRA_TIME_DEFAULT: f32 = 0.0;
const TEMP_LEAD_OFFSET_DEFAULT: i16 = 5;
const TEMP_OFFSET_DEFAULT: i16 = 0;

pub(crate) enum SyncStorageStateEnum {
    WritePid {
        pid: bool,
        pid_p: f32,
        pid_i: f32,
        pid_d: f32,
    },
    WriteTempSettings {
        wait_time: f32,
        extra_time: f32,
        temp_lead_offset: i16,
        temp_offset: i16,
    },
}

#[derive(Debug, Encode, Decode, Clone)]
pub(crate) struct StorageData {
    magic: u8,
    version: u8,
    pub pid_p: f32,
    pub pid_i: f32,
    pub pid_d: f32,
    pub pid: bool,
    pub temp_wait_time: f32,
    pub temp_extra_time: f32,
    pub temp_lead_offset: i16,
    pub temp_offset: i16,
}

impl Default for StorageData {
    fn default() -> Self {
        Self {
            magic: FLASH_MAGIC,
            version: FLASH_VERSION,
            pid_p: 0.0,
            pid_i: 0.0,
            pid_d: 0.0,
            pid: false,
            temp_wait_time: MAX_WAIT_TIME_DEFAULT,
            temp_extra_time: EXTRA_TIME_DEFAULT,
            temp_lead_offset: TEMP_LEAD_OFFSET_DEFAULT,
            temp_offset: TEMP_OFFSET_DEFAULT,
        }
    }
}

pub(crate) struct Storage<'a> {
    channel: SyncStateChannelReceiver<'a, SyncStorageStateEnum>,
    storage: StorageData,
    flash: flash::Flash<'a, embassy_rp::peripherals::FLASH, flash::Blocking, FLASH_SIZE>,
}

impl<'a> Storage<'a> {
    pub fn new(
        startup_storage: &StorageData,
        flash: flash::Flash<'a, embassy_rp::peripherals::FLASH, flash::Blocking, FLASH_SIZE>,
        channels: &'a channels::Channels,
    ) -> Self {
        Self {
            channel: channels.get_storage_rx(),
            storage: startup_storage.clone(),
            flash,
        }
    }

    pub fn flash_read(
        flash: &mut flash::Flash<'a, embassy_rp::peripherals::FLASH, flash::Blocking, FLASH_SIZE>,
    ) -> StorageData {
        let mut buf = [0; STORAGE_SIZE as usize];
        if flash.blocking_read(STORAGE_OFFSET, &mut buf).is_ok() {
            let mut storage: StorageData = bincode::decode_from_slice(&buf, BINCODE_CONFIG)
                .unwrap_or_default()
                .0;

            //sanity checks
            if storage.magic != FLASH_MAGIC {
                storage = StorageData::default();
            }
            if storage.version != FLASH_VERSION {
                storage = StorageData::default();
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
            if storage.temp_wait_time.is_nan() {
                storage.temp_wait_time = MAX_WAIT_TIME_DEFAULT;
            }
            if storage.temp_extra_time.is_nan() {
                storage.temp_extra_time = EXTRA_TIME_DEFAULT;
            }

            storage
        } else {
            StorageData::default()
        }
    }

    pub async fn flash_task(&mut self) -> ! {
        let rx = self.channel;
        loop {
            let query = rx.receive().await;
            match query {
                SyncStorageStateEnum::WritePid {
                    pid,
                    pid_p,
                    pid_i,
                    pid_d,
                } => {
                    self.storage.pid_p = pid_p;
                    self.storage.pid_i = pid_i;
                    self.storage.pid_d = pid_d;
                    self.storage.pid = pid;
                }
                SyncStorageStateEnum::WriteTempSettings {
                    wait_time,
                    extra_time,
                    temp_lead_offset,
                    temp_offset,
                } => {
                    self.storage.temp_wait_time = wait_time;
                    self.storage.temp_extra_time = extra_time;
                    self.storage.temp_lead_offset = temp_lead_offset;
                    self.storage.temp_offset = temp_offset;
                }
            }

            let mut buf = [0; STORAGE_SIZE as usize];

            bincode::encode_into_slice(&self.storage, &mut buf, BINCODE_CONFIG)
                .expect("flashtask enc fail");

            self.flash
                .blocking_erase(STORAGE_OFFSET, STORAGE_OFFSET + STORAGE_SIZE)
                .expect("flashtask erase fail");
            self.flash
                .blocking_write(STORAGE_OFFSET, &buf)
                .expect("flashtask write fail");
        }
    }
}
