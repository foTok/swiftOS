use stack_vec::StackVec;
use crate::console::{CONSOLE};

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }
}

// key code
const key_BS: u8 = 8;   //backspace
const key_DEL: u8 = 46; //delete
const key_LA: u8 = 37;  //left arrow
const key_UA: u8 = 38;  //up arrow
const key_RA: u8 = 39;  //right arrow
const key_DA: u8 = 40;  //down arrow

// ascii code
const BEL: u8 = 0x07u8;
const BS: u8 = 0x08u8;
const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const ESC: u8 = 0x1bu8;
const DEL: u8 = 0x7fu8;


/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns: it is perpetually in a shell loop.
pub fn shell(prefix: &str) -> ! {
    loop{
        let byte = read_symbol();
    }
}


fn read_symbol() -> u8{
    let console = &mut CONSOLE;
    //state = 0, 1, 2, 3, 4
    let mut state = 0;
    let mut symbol = 0u8;
    loop{
        let byte = console.read_byte();
        match state{
            0 => {
                if byte==ESC {
                    state = 1;
                }
                else{
                    symbol = byte;
                    break;
                }
            },
            1 => {
                if byte==b'['{
                    state = 2;
                }
                else{
                    symbol = BEL;
                    break;
                }
            },
            2 => {
                if byte==b'3'{
                    state = 3;
                }
                else if byte==b'A'{
                    symbol = key_UA;
                    break;
                }
                else if byte==b'B'{
                    symbol = key_DA;
                    break;
                }
                else if byte==b'C'{
                    symbol = key_RA;
                    break;
                }
                else if byte==b'D'{
                    symbol = key_LA;
                    break;
                }
                else{
                    symbol = BEL;
                    break;
                }
            },
            3 => {
                if byte==b'~'{
                    symbol = key_DEL;
                    break;
                }
                else{
                    symbol = BEL;
                    break;
                }
            },
            _ => {
                symbol = BEL;
                break;
            }
        }
    }

    match symbol {
        key_BS => {
            console.write_byte(b'b');
        },
        key_DEL => {
            console.write_byte(b'd');
        }
        key_LA => {
            console.write_byte(b'<');
        }
        key_RA => {
            console.write_byte(b'>');
        }
        key_UA => {
            console.write_byte(b'^');
        }
        key_DA => {
            console.write_byte(b'v');
        }

        _ => {
            console.write_byte(symbol);
        }
    }

    symbol
}
