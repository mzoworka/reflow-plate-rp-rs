use crate::{
    display::SyncDisplayStateEnum,
    heater::SyncHeatStateEnum,
    menu::SyncMenuStateEnum,
    storage::SyncStorageStateEnum,
    tools::{SyncStateChannel, SyncStateChannelReceiver, SyncStateChannelSender},
    watchdog::SyncWdStateEnum,
};

pub(crate) struct Channels {
    watchdog: SyncStateChannel<SyncWdStateEnum>,
    display: SyncStateChannel<SyncDisplayStateEnum>,
    heat: SyncStateChannel<SyncHeatStateEnum>,
    storage: SyncStateChannel<SyncStorageStateEnum>,
    menu: SyncStateChannel<SyncMenuStateEnum>,
}

impl Channels {
    pub fn new() -> Self {
        Self {
            watchdog: SyncStateChannel::<SyncWdStateEnum>::new(),
            display: SyncStateChannel::<SyncDisplayStateEnum>::new(),
            heat: SyncStateChannel::<SyncHeatStateEnum>::new(),
            storage: SyncStateChannel::<SyncStorageStateEnum>::new(),
            menu: SyncStateChannel::<SyncMenuStateEnum>::new(),
        }
    }

    pub fn get_watchdog_rx(&self) -> SyncStateChannelReceiver<'_, SyncWdStateEnum> {
        self.watchdog.receiver()
    }

    pub fn get_watchdog_tx(&self) -> SyncStateChannelSender<'_, SyncWdStateEnum> {
        self.watchdog.sender()
    }

    pub fn get_display_rx(&self) -> SyncStateChannelReceiver<'_, SyncDisplayStateEnum> {
        self.display.receiver()
    }

    pub fn get_display_tx(&self) -> SyncStateChannelSender<'_, SyncDisplayStateEnum> {
        self.display.sender()
    }

    pub fn get_heat_rx(&self) -> SyncStateChannelReceiver<'_, SyncHeatStateEnum> {
        self.heat.receiver()
    }

    pub fn get_heat_tx(&self) -> SyncStateChannelSender<'_, SyncHeatStateEnum> {
        self.heat.sender()
    }

    pub fn get_storage_rx(&self) -> SyncStateChannelReceiver<'_, SyncStorageStateEnum> {
        self.storage.receiver()
    }

    pub fn get_storage_tx(&self) -> SyncStateChannelSender<'_, SyncStorageStateEnum> {
        self.storage.sender()
    }

    pub fn get_menu_rx(&self) -> SyncStateChannelReceiver<'_, SyncMenuStateEnum> {
        self.menu.receiver()
    }

    pub fn get_menu_tx(&self) -> SyncStateChannelSender<'_, SyncMenuStateEnum> {
        self.menu.sender()
    }
}
