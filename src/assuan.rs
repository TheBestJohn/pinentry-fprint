use std::io::{self, BufRead, Write};

#[derive(Debug)]
pub enum Command {
    SetDesc(String),
    SetError(#[allow(dead_code)] String),
    SetKeyInfo(String),
    GetPin,
    Confirm,
    Message,
    GetInfo(String),
    Reset,
    Bye,
    Other,
}

pub struct AssuanServer {
    stdin: io::BufReader<io::Stdin>,
    stdout: io::Stdout,
}

impl AssuanServer {
    pub fn new() -> Self {
        Self {
            stdin: io::BufReader::new(io::stdin()),
            stdout: io::stdout(),
        }
    }

    pub fn send_ok(&mut self) {
        let _ = writeln!(self.stdout, "OK");
        let _ = self.stdout.flush();
    }

    pub fn send_data(&mut self, data: &str) {
        let encoded: String = data
            .chars()
            .map(|c| match c {
                '%' => "%25".to_string(),
                '\n' => "%0A".to_string(),
                '\r' => "%0D".to_string(),
                _ => c.to_string(),
            })
            .collect();
        let _ = writeln!(self.stdout, "D {encoded}");
        let _ = self.stdout.flush();
    }

    pub fn send_error(&mut self, code: u32, msg: &str) {
        let _ = writeln!(self.stdout, "ERR {code} {msg}");
        let _ = self.stdout.flush();
    }

    pub fn send_greeting(&mut self) {
        let _ = writeln!(self.stdout, "OK Pleased to meet you - pinentry-fprint");
        let _ = self.stdout.flush();
    }

    pub fn read_command(&mut self) -> Option<Command> {
        let mut line = String::new();
        match self.stdin.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => Some(parse_command(line.trim_end())),
            Err(_) => None,
        }
    }
}

fn parse_command(line: &str) -> Command {
    let (cmd, args) = line.split_once(' ').unwrap_or((line, ""));

    match cmd {
        "SETDESC" => Command::SetDesc(percent_decode(args)),
        "SETERROR" => Command::SetError(percent_decode(args)),
        "SETKEYINFO" => Command::SetKeyInfo(args.to_string()),
        "GETPIN" => Command::GetPin,
        "CONFIRM" => Command::Confirm,
        "MESSAGE" => Command::Message,
        "GETINFO" => Command::GetInfo(args.to_string()),
        "RESET" => Command::Reset,
        "BYE" => Command::Bye,
        _ => Command::Other,
    }
}

fn percent_decode(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                bytes.push(byte);
            }
        } else {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}
