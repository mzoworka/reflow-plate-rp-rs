
#[macro_export]
macro_rules! select {
    ($first:ident, $second:ident, $($rest:ident,)*) => {
        {
            let mut __select_fut = embassy_futures::select::select($first, $second);
            $(let __select_fut_next = embassy_futures::select::select(__select_fut, $rest); let __select_fut = __select_fut_next;)*
            __select_fut
        }
    };
}

#[macro_export]
macro_rules! join {
    ($first:ident, $second:ident, $($rest:ident,)*) => {
        {
            let mut __select_fut = embassy_futures::join::join($first, $second);
            $(let __select_fut_next = embassy_futures::join::join(__select_fut, $rest); let __select_fut = __select_fut_next;)*
            __select_fut
        }
    };
}

pub(crate) type BincodeConfigType =
    bincode::config::Configuration<bincode::config::LittleEndian, bincode::config::Fixint>;
pub(crate) const BINCODE_CONFIG: BincodeConfigType = bincode::config::standard()
    .with_little_endian()
    .with_fixed_int_encoding();

pub(crate) async fn wait_for_each_state<T: PartialEq, const N: usize>(states: [T; N], rx: SyncStateChannelReceiver<'_, T>) -> [T; N] {
    let mut values = states.map(|x| (x, 1));
    let mut found = 0;
    loop {
        let x = rx.receive().await;
        for i in 0..N {
            if values[i].0 == x && values[i].1 != 0 {
                values[i].1 -= 1; 
                found += 1;
                break;
            }
        }
        if found == N {
            return values.map(|x| x.0);
        }
    }
}

pub(crate) type SyncStateChannel<T> = embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::ThreadModeRawMutex, T, 4>;
pub(crate) type SyncStateChannelReceiver<'a, T> = embassy_sync::channel::Receiver<'a, embassy_sync::blocking_mutex::raw::ThreadModeRawMutex, T, 4>;
pub(crate) type SyncStateChannelSender<'a, T> = embassy_sync::channel::Sender<'a, embassy_sync::blocking_mutex::raw::ThreadModeRawMutex, T, 4>;
