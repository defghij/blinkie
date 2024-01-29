#![no_std]
#![no_main]


use panic_halt as _;
use arduino_hal::prelude::*;

use blinkie::morse;

#[allow(dead_code)]
enum ControlSeq {
    // Toggle Characters
    Enable,
    Emitter,
    Mode,
    // Control-Flow Characters
    Encode,
    Print,
    Clear,
    Halt
}
impl ControlSeq {
    const ENABLE:  char = '#';
    const EMITTER: char = '[';
    const MODE:    char = ']';
    const ENCODE:  char = '>';
    const PRINT:   char = '*';
    const CLEAR:   char = '<';
    const HALT:    char = '~';
}
/*
    This is a _Morse Code_ called **Blinkie** application for the Arduino Uno
    written for the EN.605.715 Course. Some notes about the
    application. There are a few different control characters listed
    below. The input reader for this application is non-blocking.
    That means, for example, if you are in the Continuous mode and
    input the Control Character that is may not register. You may
    need to send the command again. Otherwise, all other commands should
    operate as expected.
    Modes:
        Full Tape Readout:
            Will emit characters from the Tape once in a first to last fashion.
            This is not interruptible.
        Continuous:
            Will emit characters from the buffer in a circular and looping fashion.
            That is it will start at 0, continue to the last valid character on the tape
            and then repeat. This is interruptible via the Halt command. However,
            as noted above, the input is non-blocking so the command may need repeating.
Commands:
    '#' --Enable: enables and disables the recording of characters onto the Tape. [Toggle]
    '[' --Emitter: toggles the emitter type (LED|CONSOLE). [Toggle]
    ']' --Mode: toggles between Full Tape Readout and Continuous (or looping) modes. [Toggle]
    '>' --Encode: begins encoding and emitting of characters on the tape.
    '*' --Print: displays the tape-- a circular buffer-- with an indicator of the current writable slot.
    '<' --Clear: returns the tape to the initial state.
    '~' --Halt: stops the emitting operations when in Continuous mode.
*/

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let serial = arduino_hal::default_serial!(dp, pins, 57600);
    let led = pins.d13.into_output();

    let (mut reader, mut writer) = serial.split();

    ufmt::uwrite!(writer, "{}","Welcome to Blinkie! You may begin now:\r\n").unwrap_infallible();

    let mut enable: bool = false;
    let mut continuous: bool = false;
    let mut morse_code_machine: morse::Machine = morse::Machine::new(led, writer);

    loop {

        let c: char = match reader.read() {
            Ok(byte)               => byte as char,
            Err(nb::Error::WouldBlock) => '\0',
            Err(nb::Error::Other(_))   => '\0',
        };

        match c {
            ControlSeq::ENABLE  => { enable = !enable;                           continue; },
            ControlSeq::EMITTER => { morse_code_machine.switch_emitter();        continue; },
            ControlSeq::CLEAR   => { morse_code_machine.reset_tape();            continue; },
            ControlSeq::PRINT   => { morse_code_machine.print_tape();            continue; },
            ControlSeq::ENCODE  => { morse_code_machine.send_tape();             continue; },
            ControlSeq::MODE    => { continuous = !continuous;                   continue; },
            ControlSeq::HALT    => { continuous = false;                         continue; },
            _ => { 
                if continuous {
                    morse_code_machine.emit_and_step();
                } else
                if enable {
                    morse_code_machine.checked_insert_into_tape(c);
                }
                continue;
            }
        }
        
    }
}
