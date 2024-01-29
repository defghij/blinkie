#![no_std]
#[allow(unused_macros)]
#[allow(dead_code)]

pub mod types {
    use arduino_hal::port::mode::Output;
    use avr_hal_generic::{
        port::mode::Input,
        usart::{
            Usart,
            UsartWriter,
            UsartReader,
        }
    };
    use panic_halt as _;
    use atmega_hal::{
        port::{
            PD1,
            PD0
        }, Atmega
    };

    type Device = Atmega;
    type DeviceInterface = atmega_hal::pac::USART0;
    type InputPin = avr_hal_generic::port::Pin<Input, PD0>;
    type OutputPin = avr_hal_generic::port::Pin<Output, PD1>;
    type BaudRate = avr_hal_generic::clock::MHz16;
    pub type SerialConsole = Usart<Device, DeviceInterface, InputPin, OutputPin, BaudRate>;
    pub type SerialReader = UsartReader<Device, DeviceInterface, InputPin, OutputPin, BaudRate>;
    pub type SerialWriter = UsartWriter<Device, DeviceInterface, InputPin, OutputPin, BaudRate>;

    pub type Led = arduino_hal::port::Pin<arduino_hal::port::mode::Output, atmega_hal::port::PB5>;
}

pub mod morse {
    use super::types::{Led, SerialWriter};
    use core::slice::Iter;
    use arduino_hal::prelude::_unwrap_infallible_UnwrapInfallible;
    const UNIT: u16 = 1000 /*miliseconds*/;

    /// Enum to define the times between different elements of a Morse
    /// Code sequence. This set of timings was https://morsecode.world/international/timing.html
    /// wherein I've defined a `unit` as $1$ second.
    #[repr(u16)]
    pub enum Time {
        T1 = 1 * UNIT,
        T2 = 3 * UNIT,
        T3 = 7 * UNIT,
    } impl Time {
        pub const DOT            : u16 = Time::T1 as u16;
        pub const DASH           : u16 = Time::T2 as u16;
        pub const INTRA_SYM_GAP  : u16 = Time::T1 as u16;
        pub const INTRA_ASCII_GAP: u16 = Time::T2 as u16;
        pub const WORD_GAP       : u16 = Time::T3 as u16;
    }

    #[repr(u8)]
    pub enum Symbols {
        S1 = '.' as u8,
        S2 = '_' as u8,
    } impl Symbols {
        pub const DOT:  char = Symbols::S1 as u8 as char;
        pub const DASH: char = Symbols::S2 as u8 as char;
    }

    pub struct Sequence(&'static str);
    impl Sequence {
        #[inline(always)]
        pub fn chars(&self) -> core::str::Chars {
            self.0.chars()
        }
        #[inline(always)]
        pub fn len(&self) -> usize {
            self.0.len()
        }
    }

    pub struct Code();
    impl Code {
        #[allow(non_upper_case_globals)]
        pub const VALID_CHARACTERS: [char;55] = [
                'a',  'b',  'c',  'd', 'e',  'f',  'g',  'h',  'i',  'j',  'k',  'l',
                'm',  'n',  'o',  'p', 'q',  'r',  's',  't',  'u',  'v',  'w',  'x',
                'y',  'z',  
                // NUMERIC----------------------------------------------------------
                '0',  '1',  '2',  '3',  '4',  '5',  '6',  '7',  '8',  '9',
                // SYMBOLS----------------------------------------------------------
                '.',  ',',  '?',  '\'',  '!',  '/',  '(',  ')',  '&',  ':',  ';',  '=',
                '+',  '-',  '_',  '"',   '$',  '@',
                // Special----------------------------------------------------------
                ' '
        ];

        #[inline(always)]
        pub fn is_valid_ascii(ascii: &char) -> bool { Code::VALID_CHARACTERS.contains(ascii) }

        #[inline(always)]
        pub fn char_to_symbol(ascii: &char) -> &'static str {
            match ascii {
                // ALPHA------------------------------------------------------------
                'a' => "._",       'b' => "_...",    'c' => "_._.",    'd' => "_..",
                'e' => ".",        'f' => ".._.",    'g' => "__.",     'h' => "....",
                'i' => "..",       'j' => ".___",    'k' => "_._",     'l' => "._..", 
                'm' => "__",       'n' => "_.",      'o' => "___",     'p' => ".__.", 
                'q' => "__._",     'r' => "._.",     's' => "...",     't' => "_", 
                'u' => ".._",      'v' => "..._",    'w' => ".__",     'x' => "_.._",
                'y' => "_.__",     'z' => "__..",  
                // NUMERIC----------------------------------------------------------
                '0' => "_____",    '1' => ".____",   '2' => "..___",   '3' => "...__",
                '4' => "...._",    '5' => ".....",   '6' => "_....",   '7' => "__...",
                '8' => "___..",    '9' => "____.",
                // SYMBOLS----------------------------------------------------------
                '.' => "._._._",   ',' => "__..__",  '?' => "..__..",  '\'' => ".____.",
                '!' => "_._.__",   '/' => "_.._.",   '(' => "_.__.",   ')' => "_.__._",
                '&' => "._...",    ':' => "___...",  ';' => "_._._.",  '=' => "_..._",
                '+' => "._._.",    '-' => "_...._",  '_' => "..__._",  '"' => "._.._.",
                '$' => "..._.._",  '@' => ".__._.", 
                _ => panic!("Invalid Ascii Character encountered"),
            }
        }

    }

    pub struct CircularBuffer {
        buffer: [char; CircularBuffer::MAX_SLOTS],
        current_slot: usize,
    }
    impl CircularBuffer {
        pub const MAX_SLOTS: usize = 32;

        #[inline(always)]
        pub fn new() -> CircularBuffer {
            CircularBuffer {
               buffer: ['\0'; CircularBuffer::MAX_SLOTS],
               current_slot: 0,
            }
        }

        #[inline(always)]
        pub fn clear(&mut self) -> &mut Self {
            self.buffer = [' '; CircularBuffer::MAX_SLOTS];
            self.current_slot = 0;
            self
        }

        #[inline(always)]
        pub fn insert(&mut self, val: char) -> &mut Self {
            self.buffer[self.current_slot] = val;
            self.current_slot = (self.current_slot + 1).rem_euclid(CircularBuffer::MAX_SLOTS);
            self
        }

        #[inline(always)]
        pub fn current_slot(&self) -> usize {
            self.current_slot
        }

        #[inline(always)]
        pub fn iter(&mut self) -> Iter<'_, char> {
            self.buffer.iter()
        }

        pub fn has_unused_slots(&self) -> bool {
            let unused_slots = self.buffer.iter().filter(|c| {
                match c {
                    '\0' => true,
                    _    => false,
                }
            }).count();
            unused_slots > 0
        }

        #[inline(always)]
        pub fn debug(&mut self, serial: &mut SerialWriter) {
            let slot = self.current_slot();
            ufmt::uwrite!(serial, "Debug-------------------\r\n").unwrap_infallible();
            self.iter().enumerate().for_each(|(i,_)|   {
                if i == slot { ufmt::uwrite!(serial, "V").unwrap_infallible() }
                else         { ufmt::uwrite!(serial, " ").unwrap_infallible() }
            });
            ufmt::uwrite!(serial, "\r\n").unwrap_infallible();

            self.iter().for_each(|c| { ufmt::uwrite!(serial, "{}", c).unwrap_infallible() });

            ufmt::uwrite!(serial, "\r\n").unwrap_infallible();
            ufmt::uwrite!(serial, "------------------------\r\n").unwrap_infallible();
        }
    }

    pub struct Emitter { 
        led: Led,
        console: SerialWriter,
        emitter: EmitterKind,
    } impl Emitter {
        #[inline(always)]
        pub fn new(led: Led, console: SerialWriter) -> Emitter {
            Emitter { led, console, emitter: EmitterKind::CONSOLE }
        }

        #[inline(always)]
        fn set_emitter(&mut self, emitter: EmitterKind) -> &Self {
            self.emitter = emitter;
            self
        }   

        #[inline(always)]
        fn write(&mut self, string: &str) -> &Self {
            ufmt::uwrite!(self.console, "{}",string).unwrap_infallible();
            self
        }
        
        #[inline(always)]
        fn do_op(&mut self, op: EmitterOp) -> &mut Self {
            let op_duration = match op {
                EmitterOp::Dot          => Time::DOT,
                EmitterOp::Dash         => Time::DASH,
                EmitterOp::SymbolGap    => Time::INTRA_SYM_GAP,
                EmitterOp::CharacterGap => Time::INTRA_ASCII_GAP,
                EmitterOp::WordGap      => Time::WORD_GAP,
            };
            match self.emitter {
                EmitterKind::CONSOLE => { 
                    self.write(op.to_str());
                    self.write("\r\n");
                    arduino_hal::delay_ms(op_duration);
                },
                EmitterKind::LED     => { 
                    match op {
                        EmitterOp::Dot | EmitterOp::Dash
                            => {
                                self.led.set_high();
                                arduino_hal::delay_ms(op_duration);
                                self.led.set_low();
                            }
                        EmitterOp::WordGap | EmitterOp::SymbolGap | EmitterOp::CharacterGap 
                            => {
                                arduino_hal::delay_ms(op_duration);
                            }
                    }
                }
            }
            self 
        }

        #[inline(always)]
        fn dot(&mut self) -> &Self { self.do_op(EmitterOp::Dot) }

        #[inline(always)]
        fn dash(&mut self) -> &Self { self.do_op(EmitterOp::Dash) }

        #[inline(always)]
        fn symbol_gap(&mut self) ->&Self { self.do_op(EmitterOp::SymbolGap)  }

        #[inline(always)]
        fn character_gap(&mut self) ->&Self { self.do_op(EmitterOp::CharacterGap) }

        #[inline(always)]
        fn word_gap(&mut self) ->&Self { self.do_op(EmitterOp::WordGap) }
    }

    pub enum EmitterKind {
        LED,
        CONSOLE
    } impl EmitterKind {
        pub fn to_str(&self) -> &'static str {
            match self {
                EmitterKind::LED => "EmitterKind::LED",
                EmitterKind::CONSOLE => "EmitterKind::CONSOLE",
            }
        }
    }

    pub enum EmitterOp {
        Dot,
        Dash,
        SymbolGap,
        CharacterGap,
        WordGap
    } impl EmitterOp {
        pub fn to_str(&self) -> &'static str {
            match self {
                EmitterOp::Dot => "EmitterOp::Dot",
                EmitterOp::Dash => "EmitterOp::Dash",
                EmitterOp::SymbolGap => "EmitterOp::SymbolGap",
                EmitterOp::CharacterGap => "EmitterOp::CharacterGap",
                EmitterOp::WordGap => "EmitterOp::WordGap",
            }
        }
    }

    pub struct Machine {
        tape: CircularBuffer,
        emitter: Emitter,
        current_slot: usize,
        end_slot: usize,
    } impl Machine {
        #[inline(always)]
        pub fn new(led: Led, console: SerialWriter) -> Machine {
            let tape: CircularBuffer = CircularBuffer::new();
            let emitter: Emitter = Emitter::new(led, console);
            Machine { tape, emitter, current_slot: 0, end_slot: 0 }
        }

        #[inline(always)]
        pub fn checked_insert_into_tape(&mut self, c: char) -> &mut Self {
            if Code::is_valid_ascii(&c) {
                self.tape.insert(c);
                if self.end_slot < CircularBuffer::MAX_SLOTS { self.end_slot += 1; }
            }
            self
        }

        #[inline(always)]
        pub fn reset_tape(&mut self) -> &mut Self {
            self.tape.clear();
            self.current_slot = 0;
            self.end_slot = 0;
            self
        }

        #[inline(always)]
        pub fn send_tape(&mut self) -> &mut Self {
            self.emit();
            self
        }
        
        #[inline(always)]
        pub fn print_tape(&mut self) -> &mut Self {

            let slot = self.tape.current_slot();
            self.emitter.write("Tape-------------------\r\n");
            self.tape.iter().enumerate().for_each(|(i,_)|   {
                if i == slot { self.emitter.write("V");  }
                else         { self.emitter.write(" "); }
            });
            self.emitter.write( "\r\n");
            //let bytes: &[u8; CircularBuffer::MAX_SLOTS] = self.tape.iter().map(|c| *c as u8 ).collect();

            self.tape.iter().for_each(|c| {
                let byte: [u8; 1] = [*c as u8];
                let string = core::str::from_utf8(&byte).unwrap();
                self.emitter.write(string);
            });
            self.emitter.write("\r\n");
            self.emitter.write("------------------------\r\n");
            self
        }

        #[inline(always)]
        pub fn emit_and_step(&mut self) -> &mut Self {
            let current_slot_char: char = self.tape.buffer[self.current_slot]; 

            self.emit_character(&current_slot_char);
            if self.current_slot != self.end_slot {
                self.emitter.character_gap();
            }
            let next_slot = (self.current_slot + 1).rem_euclid(self.end_slot);
            /*
            match self.tape.buffer[next_slot] {
                ' ' => { /* Will be taken care of on next call to `emit_and_step()` */ },
                _   => { self.emitter.character_gap(); },
            }
            */
            self.current_slot = next_slot;
            self
        }
        
        #[inline(always)]
        fn emit_character(&mut self, c: &char) -> &mut Self {
            let symbols: &'static str = Code::char_to_symbol(c);                
            let num_syms: usize = symbols.len();

            symbols.chars()
                          .into_iter()
                          .enumerate()
                          .for_each(|(i,s)| {
                                if i != 0 && i != num_syms { self.emitter.symbol_gap(); }
                                match s {
                                    Symbols::DOT  => { self.emitter.dot(); return;  },
                                    Symbols::DASH => { self.emitter.dash(); return; },
                                    _             => ()
                                }
                        });
            self
        }
        
        #[inline(always)]
        pub fn switch_emitter(&mut self) -> &mut Self {
            match self.emitter.emitter {
                EmitterKind::LED     => { self.emitter.set_emitter(EmitterKind::CONSOLE); }
                EmitterKind::CONSOLE => { self.emitter.set_emitter(EmitterKind::LED);     }
            }
            self
        }   

        #[inline(always)]
        fn emit(&mut self) {
            self.tape.iter()
                  .for_each(|c| {
                      match *c {
                          '\0' => { return; },
                          ' '  => { self.emitter.word_gap(); return; },
                          _    => {
                                let symbols: &'static str = Code::char_to_symbol(c);                
                                let num_syms: usize = symbols.len();

                                symbols.chars()
                                              .into_iter()
                                              .enumerate()
                                              .for_each(|(i,s)| {
                                                    if i != 0 && i != num_syms { self.emitter.symbol_gap(); }
                                                    match s {
                                                        Symbols::DOT  => { self.emitter.dot(); return;  },
                                                        Symbols::DASH => { self.emitter.dash(); return; },
                                                        _             => ()
                                                    }
                                            });
                                self.emitter.character_gap();
                          }
                      }
            });
        }
    }
}
