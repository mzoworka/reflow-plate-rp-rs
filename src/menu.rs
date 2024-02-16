use embassy_rp::gpio::Input;
use embassy_time::Timer;
use simplestaticstring::{format_static, StaticString};
use core::fmt::Write;

use crate::{temperature, storage::{Storage, SyncStorageStateEnum}, display::SyncDisplayStateEnum, heater::SyncHeatStateEnum, tools::SyncStateChannelSender};

//traits
trait MenuItemTextTrait {
    fn get(&self, menu: &Menu) -> StaticString<20>;
}

trait MenuItemActionTrait {
    fn call(&self, btn: u8, amount: u8, menu: &mut Menu) -> MenuItemAction;
}

enum MenuItemText {
    Static(&'static str),
    Render(&'static dyn MenuItemTextTrait),
}

//item types
enum MenuItemAction {
    None,
    MovePositionUp,
    MovePositionDown,
    OpenMenu(&'static &'static MenuType),
    Back,
    Custom(&'static dyn MenuItemActionTrait),
}

struct MenuItem {
    text: MenuItemText,
    action: MenuItemAction,
}

type MenuType = [MenuItem];

struct MenuItemTargetTempStatic{}
impl MenuItemTextTrait for MenuItemTargetTempStatic {
    fn get(&self, menu: &Menu) -> StaticString<20> {
        format_static!("Static temp: {:03}", menu.target_temp.0)
    }
}

impl MenuItemActionTrait for MenuItemTargetTempStatic {
    fn call(&self, btn: u8, amount: u8, menu: &mut Menu) -> MenuItemAction {
        match btn {
            1 => {
                menu.target_temp.0 += amount as u16;
                menu.target_temp.1 = true;
                MenuItemAction::None
            },
            2 => {
                menu.profile.0 = temperature::TemperatureProfileEnum::Static;
                menu.profile.1 = true;
                MenuItemAction::Back
            },
            3 => {
                menu.target_temp.0 -= amount as u16;
                menu.target_temp.1 = true;
                MenuItemAction::None
            },
            _ => MenuItemAction::None,

        }
    }
}

struct MenuItemTargetTempProfileA{}
impl MenuItemTextTrait for MenuItemTargetTempProfileA {
    fn get(&self, menu: &Menu) -> StaticString<20> {
        format_static!("Peak temp(A): {:03}", menu.target_temp.0)
    }
}

impl MenuItemActionTrait for MenuItemTargetTempProfileA {
    fn call(&self, btn: u8, amount: u8, menu: &mut Menu) -> MenuItemAction {
        match btn {
            1 => {
                menu.target_temp.0 += amount as u16;
                menu.target_temp.1 = true;
                MenuItemAction::None
            },
            2 => {
                menu.profile.0 = temperature::TemperatureProfileEnum::ProfileA;
                menu.profile.1 = true;
                MenuItemAction::Back
            },
            3 => {
                menu.target_temp.0 -= amount as u16;
                menu.target_temp.1 = true;
                MenuItemAction::None
            },
            _ => MenuItemAction::None,

        }
    }
}

struct MenuItemPidP{}
impl MenuItemTextTrait for MenuItemPidP {
    fn get(&self, menu: &Menu) -> StaticString<20> {
        format_static!("Set P: {:03.02}", menu.pid_p.0)
    }
}

impl MenuItemActionTrait for MenuItemPidP {
    fn call(&self, btn: u8, amount: u8, menu: &mut Menu) -> MenuItemAction {
        match btn {
            1 => {
                menu.pid_p.0 += 0.01 * (amount as f32);
                menu.pid_p.1 = true;
                MenuItemAction::None
            },
            2 => MenuItemAction::Back,
            3 => {
                menu.pid_p.0 -= 0.01 * (amount as f32);
                menu.pid_p.1 = true;
                MenuItemAction::None
            },
            _ => MenuItemAction::None,

        }
    }
}

struct MenuItemPidI{}
impl MenuItemTextTrait for MenuItemPidI {
    fn get(&self, menu: &Menu) -> StaticString<20> {
        format_static!("Set I: {:03.02}", menu.pid_i.0)
    }
}

impl MenuItemActionTrait for MenuItemPidI {
    fn call(&self, btn: u8, amount: u8, menu: &mut Menu) -> MenuItemAction {
        match btn {
            1 => {
                menu.pid_i.0 += 0.1 * (amount as f32);
                menu.pid_i.1 = true;
                MenuItemAction::None
            },
            2 => MenuItemAction::Back,
            3 => {
                menu.pid_i.0 -= 0.1 * (amount as f32);
                menu.pid_i.1 = true;
                MenuItemAction::None
            },
            _ => MenuItemAction::None,

        }
    }
}


struct MenuItemPidD{}
impl MenuItemTextTrait for MenuItemPidD {
    fn get(&self, menu: &Menu) -> StaticString<20> {
        format_static!("Set D: {:03.02}", menu.pid_d.0)
    }
}

impl MenuItemActionTrait for MenuItemPidD {
    fn call(&self, btn: u8, amount: u8, menu: &mut Menu) -> MenuItemAction {
        match btn {
            1 => {
                menu.pid_d.0 += 0.1 * (amount as f32);
                menu.pid_d.1 = true;
                MenuItemAction::None
            },
            2 => MenuItemAction::Back,
            3 => {
                menu.pid_d.0 -= 0.1 * (amount as f32);
                menu.pid_d.1 = true;
                MenuItemAction::None
            },
            _ => MenuItemAction::None,

        }
    }
}

struct MenuItemPidUsePid{}
impl MenuItemTextTrait for MenuItemPidUsePid {
    fn get(&self, menu: &Menu) -> StaticString<20> {
        format_static!("Use Pid: {}", match menu.pid.0 {
            true => "true",
            false => "false",
        } )
    }
}

impl MenuItemActionTrait for MenuItemPidUsePid {
    fn call(&self, btn: u8, _amount: u8, menu: &mut Menu) -> MenuItemAction {
        match btn {
            1 => MenuItemAction::MovePositionUp,
            2 => {
                menu.pid.0 = !menu.pid.0;
                menu.pid.1 = true;
                MenuItemAction::None
            },
            3 => MenuItemAction::MovePositionDown,
            _ => MenuItemAction::None,

        }
    }
}

//menus
const MENU_TOP: &MenuType = &[
    MenuItem{text: MenuItemText::Static("Menu"), action: MenuItemAction::OpenMenu(&MENU_MAIN)},
];

const MENU_MAIN: &MenuType = &[
    MenuItem{text: MenuItemText::Static("Target temp"), action: MenuItemAction::OpenMenu(&MENU_TARGET_TEMP)},
    MenuItem{text: MenuItemText::Static("Pid"), action: MenuItemAction::OpenMenu(&MENU_PID)},
    MenuItem{text: MenuItemText::Static("Back"), action: MenuItemAction::Back},
];


const MENU_TARGET_TEMP: &MenuType = &[
    MenuItem{text: MenuItemText::Static("Static target temp"), action: MenuItemAction::OpenMenu(&MENU_TARGET_TEMP_STATIC)},
    MenuItem{text: MenuItemText::Static("Temp profile A"), action: MenuItemAction::OpenMenu(&MENU_TARGET_TEMP_PROFILE_A)},
    MenuItem{text: MenuItemText::Static("Back"), action: MenuItemAction::Back},
];

const MENU_TARGET_TEMP_STATIC: &MenuType = &[
    MenuItem{text: MenuItemText::Render(&MenuItemTargetTempStatic{}), action: MenuItemAction::Custom(&MenuItemTargetTempStatic{})},
];

const MENU_TARGET_TEMP_PROFILE_A: &MenuType = &[
    MenuItem{text: MenuItemText::Render(&MenuItemTargetTempProfileA{}), action: MenuItemAction::Custom(&MenuItemTargetTempProfileA{})},
];

const MENU_PID: &MenuType = &[
    MenuItem{text: MenuItemText::Render(&MenuItemPidUsePid{}), action: MenuItemAction::Custom(&MenuItemPidUsePid{})},
    MenuItem{text: MenuItemText::Static("Set P"), action: MenuItemAction::OpenMenu(&MENU_PID_P)},
    MenuItem{text: MenuItemText::Static("Set I"), action: MenuItemAction::OpenMenu(&MENU_PID_I)},
    MenuItem{text: MenuItemText::Static("Set D"), action: MenuItemAction::OpenMenu(&MENU_PID_D)},
    MenuItem{text: MenuItemText::Static("Back"), action: MenuItemAction::Back},
];

const MENU_PID_P: &MenuType = &[
    MenuItem{text: MenuItemText::Render(&MenuItemPidP{}), action: MenuItemAction::Custom(&MenuItemPidP{})},
];

const MENU_PID_I: &MenuType = &[
    MenuItem{text: MenuItemText::Render(&MenuItemPidI{}), action: MenuItemAction::Custom(&MenuItemPidI{})},
];

const MENU_PID_D: &MenuType = &[
    MenuItem{text: MenuItemText::Render(&MenuItemPidD{}), action: MenuItemAction::Custom(&MenuItemPidD{})},
];

//menu struct
pub(crate) struct Menu{
    menu: &'static MenuType,
    position: u8,
    target_temp: (u16, bool),
    profile: (temperature::TemperatureProfileEnum, bool),
    pid: (bool, bool),
    pid_p: (f32, bool),
    pid_i: (f32, bool),
    pid_d: (f32, bool),
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            menu: &MENU_TOP,
            position: 0u8,
            target_temp: (0, false),
            profile: (temperature::TemperatureProfileEnum::Static, false),
            pid: (false, false),
            pid_p: (0.0, false),
            pid_i: (0.0, false),
            pid_d: (0.0, false),
        }
    }
}

impl Menu {  
    pub(crate) fn render(&self) -> StaticString<60> {
        let mut output = StaticString::default();
        for pos in 0..self.menu.len() {
            let item = &self.menu[pos];
            if self.position == pos as u8 {
                match item.action {
                    MenuItemAction::Custom(_) => if output.try_extend_from_slice(b"-").is_err() {break;}
                    _ => if output.try_extend_from_slice(b">").is_err() {break;}
                }
            } else {
                if output.try_extend_from_slice(b" ").is_err() {break;}
            }

            match item.text {
                MenuItemText::Static(x) => if output.try_extend_from_slice(x.as_bytes()).is_err() {break;},
                MenuItemText::Render(x) => if output.try_extend_from_slice(x.get(&self).as_bytes()).is_err() {break;},
            };

            if output.try_extend_from_slice(b"\n").is_err() {break;}
        }

        output
    }

    fn execute_action(&mut self, action: &MenuItemAction) {
        match action {
            MenuItemAction::None => {},
            MenuItemAction::MovePositionUp => {
                if self.position == 0 {
                    self.position = (self.menu.len() - 1) as u8;
                } else {
                    self.position -= 1;
                }
            },
            MenuItemAction::MovePositionDown => {
                if self.position == (self.menu.len() - 1) as u8 {
                    self.position = 0;
                } else {
                    self.position += 1;
                }
            },
            MenuItemAction::OpenMenu(x) => {
                self.menu = x;
                self.position = 0u8;
            },
            MenuItemAction::Back => {
                self.menu = MENU_TOP;
                self.position = 0u8;
            },
            MenuItemAction::Custom(_) => {},
        }
    }

    pub(crate) fn on_up(&mut self, amount: u8) {
        let item = &self.menu[self.position as usize];
        match &item.action {
            MenuItemAction::Custom(x) => {
                let action = x.call(1, amount, self);
                self.execute_action(&action);
            },
            _ => self.execute_action(&MenuItemAction::MovePositionUp),
        }
    }
    
    pub(crate) fn on_down(&mut self, amount: u8) {
        let item = &self.menu[self.position as usize];
        match &item.action {
            MenuItemAction::Custom(x) => {
                let action = x.call(3, amount, self);
                self.execute_action(&action);
            },
            _ => self.execute_action(&MenuItemAction::MovePositionDown),
        }
    }
    
    pub(crate) fn on_enter(&mut self) {
        let item = &self.menu[self.position as usize];
        match &item.action {
            MenuItemAction::Custom(x) => {
                let action = x.call(2, 1, self);
                self.execute_action(&action);
            },
            x => self.execute_action(x),
        }
    }

    async fn send_updates(&mut self, display_tx: SyncStateChannelSender<'_, SyncDisplayStateEnum>, heat_tx: SyncStateChannelSender<'_, SyncHeatStateEnum>, storage_tx: SyncStateChannelSender<'_, SyncStorageStateEnum>) {
        if self.target_temp.1 || self.profile.1 {
            heat_tx.send(SyncHeatStateEnum::TargetTemp(self.target_temp.0, self.profile.0)).await;
            display_tx.send(SyncDisplayStateEnum::PeakTargetTemp(self.target_temp.0, self.profile.0)).await;
        }
        
        if self.pid.1 || self.pid_p.1 || self.pid_i.1 || self.pid_d.1 {
            heat_tx.send(SyncHeatStateEnum::Pid((self.pid.0, self.pid_p.0, self.pid_i.0, self.pid_d.0))).await;
            storage_tx.send(SyncStorageStateEnum::WritePid((self.pid.0, self.pid_p.0, self.pid_i.0, self.pid_d.0))).await;
        }
        
        self.target_temp.1 = false;
        self.profile.1 = false;
        self.pid_p.1 = false;
        self.pid_i.1 = false;
        self.pid_d.1 = false;
        
    }
    
}

pub(crate) async fn btn_task(startup_storage: &Storage, btn1: &'_ mut Input<'_, embassy_rp::peripherals::PIN_2>, btn2: &'_ mut Input<'_, embassy_rp::peripherals::PIN_3>, btn3: &'_ mut Input<'static, embassy_rp::peripherals::PIN_4>, display_tx: SyncStateChannelSender<'_, SyncDisplayStateEnum>, heat_tx: SyncStateChannelSender<'_, SyncHeatStateEnum>, storage_tx: SyncStateChannelSender<'_, SyncStorageStateEnum>) -> ! {
    const DEFAULT_BTN_DELAY: u8 = 10;

    let mut menu = Menu::default();
    menu.pid.0 = startup_storage.pid;
    menu.pid_p.0 = startup_storage.pid_p;
    menu.pid_i.0 = startup_storage.pid_i;
    menu.pid_d.0 = startup_storage.pid_d;

    display_tx.send(SyncDisplayStateEnum::Status(menu.render())).await;
    
    let mut last_action: u8 = 0;
    let mut counter: u8 = 0;
    let mut delay: u8 = DEFAULT_BTN_DELAY;
    let mut delay_counter: u8 = 0;
    loop {
        let time_begin = embassy_time::Instant::now();
        let debounce = time_begin.checked_add(embassy_time::Duration::from_millis(100)).unwrap_or(time_begin);

        let f1 = btn1.wait_for_falling_edge();
        let f2 = btn2.wait_for_falling_edge();
        let f3 = btn3.wait_for_falling_edge();
        let f4 = Timer::at(debounce);
        let sel_fut = crate::select!(f1, f2, f3, f4,);
        let action = match sel_fut.await {
            embassy_futures::select::Either::First(embassy_futures::select::Either::First(embassy_futures::select::Either::First(_btn1))) => {
                if last_action != 0 {
                    0
                } else {
                    counter = 0;
                    delay = DEFAULT_BTN_DELAY;
                    1
                }
            },
            embassy_futures::select::Either::First(embassy_futures::select::Either::First(embassy_futures::select::Either::Second(_btn2))) => {
                if last_action != 0 {
                    0
                } else {
                    counter = 0;
                    delay = DEFAULT_BTN_DELAY;
                    2
                }
            },
            embassy_futures::select::Either::First(embassy_futures::select::Either::Second(_btn3)) => {
                if last_action != 0 {
                    0
                } else {
                    counter = 0;
                    delay = DEFAULT_BTN_DELAY;
                    3
                }
            },
            embassy_futures::select::Either::Second(_delay) => {
                let delay_action = if btn1.is_low() {
                    1
                } else if btn2.is_low() {
                    2
                } else if btn3.is_low() {
                    3
                } else {
                    0
                };

                if delay_action != 0 {
                    counter += 1;
                    if counter > 2 {
                        counter = 0;
                        if delay > 0 {
                            delay -= 1;
                        }
                    }
                    delay_counter += 1;
                    if delay_counter > delay {
                        delay_counter = 0;
                    }
                } else {
                    counter = 0;
                    delay = DEFAULT_BTN_DELAY;        
                }

                delay_action
            },
        };

        Timer::at(debounce).await;

        last_action = action;

        let amount = match delay {
            9.. => 1,
            7.. => 2,
            5.. => 5,
            2.. => 10,
            0.. => 20,
        };

        match action {
            1 => menu.on_up(amount),
            2 => {
                menu.on_enter();
                menu.send_updates(display_tx, heat_tx, storage_tx).await;
            },
            3 => menu.on_down(amount),
            _ => {},
        };

        if action != 0 {
            display_tx.send(SyncDisplayStateEnum::Status(menu.render())).await;    
        }
    }
}
