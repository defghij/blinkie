#![no_std]
#![no_main]


use panic_halt as _;
use arduino_hal::prelude::*;

use blinkie::{
    types::{
        Led,
        SerialConsole
    },
    morse
};



enum ControlSeq {}
impl ControlSeq {
    // Toggle Controls
    pub const ENABLE: char = '#';
    pub const EMITTER:  char = '[';

    // Single-Character Controls
    pub const ENCODE: char = '>';
    pub const DEBUG:  char = '*';
    pub const CLEAR:  char = '<';
}


#[arduino_hal::entry]
fn main() -> ! {
    // Get foundational types.
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Set up pins and serial.
    // Serial: Pulls bytes from serial connection one byte at a time.
    //      Will only pull 3 bytes at consecutively before dropping 
    //      the remainder of the buffer.
    let mut serial: SerialConsole = arduino_hal::default_serial!(dp, pins, 57600);
    let led:              Led = pins.d13.into_output();
    
    ufmt::uwriteln!(&mut serial, "Hello from Arduino!\r").unwrap_infallible();
    
    let mut enable: bool = false;
    let (mut reader, writer) = serial.split();
    let mut morse_code_machine: morse::Machine = morse::Machine::new(led, writer);


    loop {
        let c: char = reader.read().unwrap() as char;
        
        //let c = nb::block!(serial.read()).unwrap_infallible() as char;
        match c {
            ControlSeq::ENABLE  => { enable = !enable;                           continue; },
            ControlSeq::EMITTER => { morse_code_machine.switch_emitter();        continue; },
            ControlSeq::CLEAR   => { morse_code_machine.reset_tape();            continue; },
            ControlSeq::DEBUG   => { morse_code_machine.print_tape();            continue; },
            ControlSeq::ENCODE  => { morse_code_machine.send_tape();             continue; },
            _ => { 
                if enable { morse_code_machine.insert_into_tape(c); }         continue; }
        }
        
    }
}
