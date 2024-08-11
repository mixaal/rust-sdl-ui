use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    sync::{Arc, RwLock},
    thread,
};

const JOY_AXIS_MIN_VALUE: i16 = -32767;
const JOY_AXIS_MAX_VALUE: i16 = 32767;
const JOY_AXIS_RANGE: f32 = JOY_AXIS_MAX_VALUE as f32 - JOY_AXIS_MIN_VALUE as f32;

#[macro_export]
macro_rules! collection {
// map-like
($($k:expr => $v:expr),* $(,)?) => {{
    use std::iter::{Iterator, IntoIterator};
    Iterator::collect(IntoIterator::into_iter([$(($k, $v),)*]))
}};
// set-like
($($v:expr),* $(,)?) => {{
    use std::iter::{Iterator, IntoIterator};
    Iterator::collect(IntoIterator::into_iter([$($v,)*]))
}};
}

lazy_static! {
    pub static ref XBOX_MAPPING: Mapping = Mapping {
        buttons: collection![0x0=>Buttons::A, 0x1=>Buttons::B, 0x2=>Buttons::X, 0x3=>Buttons::Y, 0x4=>Buttons::LB, 0x5=>Buttons::RB, 0x6=>Buttons::START, 0x7=>Buttons::SELECT],
        axes: collection![0x1=>Axes::LY, 0x0=>Axes::LX, 0x2=>Axes::LT, 0x3=>Axes::RX, 0x4=>Axes::RY, 0x5=>Axes::RT, 0x6=>Axes::Horiz, 0x7=>Axes::Vert]
    };
}

#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum Buttons {
    A = 0x0,
    B = 0x1,
    X = 0x2,
    Y = 0x3,
    LB = 0x4,
    RB = 0x5,
    SELECT = 0x6,
    START = 0x7,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy)]
enum Axes {
    LY = 0x0,
    LX = 0x1,
    LT = 0x2,
    RY = 0x3,
    RX = 0x4,
    RT = 0x5,
    Horiz = 0x6,
    Vert = 0x7,
}

#[derive(Debug, Clone)]
pub struct Mapping {
    axes: HashMap<u8, Axes>,
    buttons: HashMap<u8, Buttons>,
}

fn pct(v: i16) -> f32 {
    ((v as f32) - (JOY_AXIS_MIN_VALUE as f32)) / JOY_AXIS_RANGE
}

fn min_max(v: i16) -> f32 {
    2.0 * (((v as f32) - (JOY_AXIS_MIN_VALUE as f32)) / JOY_AXIS_RANGE) - 1.0
}

#[derive(Debug, Clone)]
pub struct GamepadState {
    buttons: [bool; 8],
    axes: [i16; 8],
}

impl GamepadState {
    pub fn initial() -> Self {
        let mut axes: [i16; 8] = [0; 8];
        axes[Axes::LT as usize] = JOY_AXIS_MIN_VALUE;
        axes[Axes::RT as usize] = JOY_AXIS_MIN_VALUE;
        Self {
            buttons: [false; 8],
            axes,
        }
    }

    pub fn button_clicked(&self, button: Buttons, last_state: &GamepadState) -> bool {
        let b_idx = button as usize;
        let current_state = self.buttons[b_idx];
        let last_state = last_state.buttons[b_idx];
        //tracing::debug!(b_idx, current_state, last_state, "button_clicked");
        last_state && !current_state
    }

    pub fn a(&self) -> bool {
        self.buttons[Buttons::A as usize]
    }

    pub fn b(&self) -> bool {
        self.buttons[Buttons::B as usize]
    }

    pub fn x(&self) -> bool {
        self.buttons[Buttons::X as usize]
    }

    pub fn y(&self) -> bool {
        self.buttons[Buttons::Y as usize]
    }

    pub fn rb(&self) -> bool {
        self.buttons[Buttons::RB as usize]
    }

    pub fn lb(&self) -> bool {
        self.buttons[Buttons::LB as usize]
    }

    pub fn start(&self) -> bool {
        self.buttons[Buttons::START as usize]
    }

    pub fn select(&self) -> bool {
        self.buttons[Buttons::SELECT as usize]
    }

    pub fn rt(&self, sensitivity: f32) -> f32 {
        pct(self.axes[Axes::RT as usize]) * sensitivity
    }

    pub fn lt(&self, sensitivity: f32) -> f32 {
        pct(self.axes[Axes::LT as usize]) * sensitivity
    }

    pub fn horiz(&self) -> f32 {
        min_max(self.axes[Axes::Horiz as usize])
    }

    pub fn vert(&self) -> f32 {
        min_max(self.axes[Axes::Vert as usize])
    }

    pub fn l_stick(&self, sensitivity: f32) -> (f32, f32) {
        let x = min_max(self.axes[Axes::LX as usize]) * sensitivity;
        let y = min_max(self.axes[Axes::LY as usize]) * sensitivity;
        (x, y)
    }

    pub fn r_stick(&self, sensitivity: f32) -> (f32, f32) {
        let x = min_max(self.axes[Axes::RX as usize]) * sensitivity;
        let y = min_max(self.axes[Axes::RY as usize]) * sensitivity;
        (x, y)
    }
}

struct GamepadStateUpdater {
    state: RwLock<GamepadState>,
    mapping: Mapping,
}

impl GamepadStateUpdater {
    fn new(mapping: Mapping) -> Self {
        Self {
            state: RwLock::new(GamepadState::initial()),
            mapping,
        }
    }

    fn update(&self, evt: &JsEvent) {
        match evt.kind {
            EventType::JsEventButton => {
                let mut g = self.state.write().unwrap();
                let mapping = self.mapping.buttons.get(&evt.number);
                if mapping.is_some() {
                    let idx = *mapping.unwrap() as usize;
                    g.buttons[idx] = evt.value == 0x1;
                }
            }
            EventType::JsEventAxis => {
                let mut g = self.state.write().unwrap();
                let mapping = self.mapping.axes.get(&evt.number);
                if mapping.is_some() {
                    let idx = *mapping.unwrap() as usize;
                    g.axes[idx] = evt.value;
                }
            }
            EventType::JsEventButtonInit => {}
            EventType::JsEventAxesInit => {}
            EventType::JsEventUknown => {}
        }
    }

    fn get(&self) -> GamepadState {
        let g = self.state.read().unwrap();
        g.clone()
    }
}

#[derive(Debug, Clone)]
enum EventType {
    JsEventButton,     /* button pressed/released */
    JsEventAxis,       /* joystick moved */
    JsEventButtonInit, /* initial state of device */
    JsEventAxesInit,   /* initial state of device */
    JsEventUknown,
}

// see /usr/include/linux/joystick.h
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JsEvent {
    time: u32,       /* event timestamp in milliseconds */
    value: i16,      /* value */
    kind: EventType, /* event type */
    number: u8,      /* axis/button number */
}

pub struct Gamepad {
    inner: Arc<GamepadInner>,
}

impl Gamepad {
    pub fn new(device: &str, mapping: Mapping) -> Self {
        Self {
            inner: Arc::new(GamepadInner::new(device, mapping)),
        }
    }

    pub fn background_handler(&self) {
        let self_local = self.inner.clone();
        thread::spawn(move || loop {
            let evt = self_local.read_event();
            self_local.updater.update(&evt);
        });
    }

    pub fn state(&self) -> GamepadState {
        self.inner.updater.get()
    }
}

struct GamepadInner {
    js: File,
    pub(crate) updater: GamepadStateUpdater,
}

impl GamepadInner {
    fn new(device: &str, mapping: Mapping) -> Self {
        let js = File::open(device).expect("can't open file");
        Self {
            js,
            updater: GamepadStateUpdater::new(mapping),
        }
    }
    fn read_event(&self) -> JsEvent {
        let mut js = &self.js;
        let mut buff: [u8; 8] = [0; 8];
        let r = js.read(&mut buff);
        if r.is_err() {
            panic!(
                "{}",
                format!("error reading event from joystick file: {}", r.unwrap_err())
            );
        }
        let nbytes = r.unwrap();
        if nbytes != 8 {
            panic!("expected to read 8 bytes of data");
        }
        // println!("buffer={:?}", buff);
        let ev_type = match buff[6] {
            0x1 => EventType::JsEventButton,
            0x2 => EventType::JsEventAxis,
            0x81 => EventType::JsEventButtonInit,
            0x82 => EventType::JsEventAxesInit,
            _ => EventType::JsEventUknown,
        };
        JsEvent {
            time: u32::from_le_bytes(buff[0..=3].try_into().unwrap()),
            value: i16::from_le_bytes(buff[4..=5].try_into().unwrap()),
            kind: ev_type,
            number: buff[7],
        }
    }
}
