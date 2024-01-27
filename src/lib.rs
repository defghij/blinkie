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
        pub const DOT            : u16 = Time::T2 as u16;
        pub const DASH           : u16 = Time::T1 as u16;
        pub const INTRA_SYM_GAP  : u16 = Time::T1 as u16;
        pub const INTRA_ASCII_GAP: u16 = Time::T2 as u16;
        pub const WORD_GAP       : u16 = Time::T3 as u16;
        pub fn dot() -> Time { Time::T1 }
        pub fn dash() -> Time {Time::T2 }
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
        pub fn chars(&self) -> core::str::Chars {
            self.0.chars()
        }
        pub fn len(&self) -> usize {
            self.0.len()
        }
    }

    pub struct Code();
    impl Code {
        #[allow(non_upper_case_globals)]
        pub const VALID_CHARACTERS: [char;54] = [
                'a',  'b',  'c',  'd', 'e',  'f',  'g',  'h',  'i',  'j',  'k',  'l',
                'm',  'n',  'o',  'p', 'q',  'r',  's',  't',  'u',  'v',  'w',  'x',
                'y',  'z',  
                // NUMERIC----------------------------------------------------------
                '0',  '1',  '2',  '3',  '4',  '5',  '6',  '7',  '8',  '9',
                // SYMBOLS----------------------------------------------------------
                '.',  ',',  '?',  '\'',  '!',  '/',  '(',  ')',  '&',  ':',  ';',  '=',
                '+',  '-',  '_',  '"',   '$',  '@',
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

        pub fn new() -> CircularBuffer {
            
            CircularBuffer {
               buffer: [' '; CircularBuffer::MAX_SLOTS],
               current_slot: 0,
            }
        }

        pub fn clear(&mut self) -> &mut Self {
            self.buffer = [' '; CircularBuffer::MAX_SLOTS];
            self.current_slot = 0;
            self
        }

        pub fn insert(&mut self, val: char) -> &mut Self {
            self.current_slot = (self.current_slot + 1).rem_euclid(CircularBuffer::MAX_SLOTS);
            self.buffer[self.current_slot] = val;
            self
        }

        pub fn current_slot(&self) -> usize {
            self.current_slot
        }

        pub fn iter(&mut self) -> Iter<'_, char> {
            self.buffer.iter()
        }

        pub fn debug(&mut self, serial: &mut SerialWriter) {
            let slot = self.current_slot();
            ufmt::uwrite!(serial, "Debug-------------------\r\n").unwrap_infallible();
            self.iter().enumerate().for_each(|(i,_)|   {
                if i == slot { ufmt::uwrite!(serial, "V").unwrap_infallible() }
                else         { ufmt::uwrite!(serial, " ").unwrap_infallible() }
            });
            ufmt::uwrite!(serial, "\r\n").unwrap_infallible();

            self.iter().for_each(|c| {
                ufmt::uwrite!(serial, "{}", c).unwrap_infallible()
            });
            ufmt::uwrite!(serial, "\r\n").unwrap_infallible();
            ufmt::uwrite!(serial, "------------------------\r\n").unwrap_infallible();
        }
    }

    pub struct Emitter { 
        led: Led,
        console: SerialWriter,
        emitter: EmitterKind,
    } impl Emitter {
        pub fn new(led: Led, console: SerialWriter) -> Emitter {
            Emitter { led, console, emitter: EmitterKind::LED }
        }
        fn set_emitter(&mut self, emitter: EmitterKind) -> &Self {
            self.emitter = emitter;
            self
        }   
        fn write(&mut self, string: &str) -> &Self {
            ufmt::uwrite!(self.console, "{}",string).unwrap_infallible();
            self
        }
        
        fn flash(&mut self, duration: u16) -> &Self {
            self.led.set_high();
            arduino_hal::delay_ms(duration);
            self.led.set_high();
            self
        }   

        fn dot(&mut self) -> &Self {
            match self.emitter {
                EmitterKind::LED => { self.write("SYMBOL::DOT\r\n"); },
                EmitterKind::CONSOLE => { self.flash(Time::DOT); }
            }   
            self
        }

        fn dash(&mut self) -> &Self {
            match self.emitter {
                EmitterKind::LED => { self.write("SYMBOL::DASH\r\n"); },
                EmitterKind::CONSOLE => { self.flash(Time::DASH); }
            }   
            self
        }
        fn symbol_gap(&mut self) ->&Self {
            match self.emitter {
                EmitterKind::LED => { self.write("GAP::SYMBOL\r\n"); },
                EmitterKind::CONSOLE => { self.flash(Time::INTRA_SYM_GAP); }
            }   
            self
        }
        fn character_gap(&mut self) ->&Self {
            match self.emitter {
                EmitterKind::LED => { self.write("GAP::CHARACTER\r\n"); },
                EmitterKind::CONSOLE => { self.flash(Time::INTRA_ASCII_GAP); }
            }   
            self
        }
        fn word_gap(&mut self) ->&Self {
            match self.emitter {
                EmitterKind::LED => { self.write("GAP::WORD\r\n"); },
                EmitterKind::CONSOLE => { self.flash(Time::WORD_GAP); }
            }   
            self
        }
    }

    pub enum EmitterKind {
        LED,
        CONSOLE
    }   

    pub struct Machine {
        tape: CircularBuffer,
        emitter: Emitter,
    } impl Machine {
        pub fn new(led: Led, console: SerialWriter) -> Machine {
            let tape: CircularBuffer = CircularBuffer::new();
            let mut emitter: Emitter = Emitter::new(led, console);
            emitter.write("[I] Morse Code Machine Created");

            Machine { tape, emitter }
        }
        pub fn insert_into_tape(&mut self, c: char) -> &mut Self {
            self.emitter.write("[I] Inserting Charater Into Tape");
            if Code::is_valid_ascii(&c) { self.tape.insert(c); }
            self
        }
        pub fn reset_tape(&mut self) -> &mut Self {
            self.emitter.write("[I] Reset Tape");
            self.tape.clear();
            self
        }
        pub fn send_tape(&mut self) -> &mut Self {
            self.emitter.write("[I] Send Tape");
            self.emit();
            self
        }
        pub fn print_tape(&mut self) -> &mut Self {
            self.emitter.write("[I] Print Tape");
            let slot = self.tape.current_slot();
            self.emitter.write("Debug-------------------\r\n");
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
        
        pub fn get_emitter(&self) -> &Emitter {
            &self.emitter
        }
        pub fn switch_emitter(&mut self) ->&Self {
            self.emitter.write("Changing Emitter");
            match self.emitter.emitter {
                EmitterKind::LED     => { self.emitter.set_emitter(EmitterKind::CONSOLE); self }
                EmitterKind::CONSOLE => { self.emitter.set_emitter(EmitterKind::LED); self }
            }
        }   

        fn emit(&mut self) {
            self.tape.iter()
                  .for_each(|c| {
                        if *c == ' ' {  self.emitter.word_gap(); return; }

                        let symbols: &'static str = Code::char_to_symbol(c);                
                        let num_syms: usize = symbols.len();

                        symbols.chars()
                                      .into_iter()
                                      .enumerate()
                                      .for_each(|(i,s)| {
                                            if i != 0 && i != num_syms { self.emitter.symbol_gap(); }
                                            match s {
                                                Symbols::DOT  => { self.emitter.dot(); return; },
                                                Symbols::DASH => { self.emitter.dash(); return; },
                                                _             => ()
                                            }
                                    });
                        self.emitter.character_gap();
            });
        }


    }
}
