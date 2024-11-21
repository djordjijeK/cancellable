extern crate cancellable;

use std::io::{Read, Write};
use std::time::Duration;
use std::{io, net::TcpListener};

use cancellable::Cancellable;
use cancellable::LoopStep;

struct Service {
    tcp_listener: TcpListener,
}

impl Cancellable for Service {
    type Error = io::Error;

    fn execute(&mut self) -> Result<LoopStep, Self::Error> {
        let (mut tcp_stream, _) = match self.tcp_listener.accept() {
            Ok((tcp_stream, socket_address)) => (tcp_stream, socket_address),
            Err(error) => return Err(error),
        };

        let _ = tcp_stream.write_all(b"$ ");
        let _ = tcp_stream.flush();

        let mut buffer = Vec::new();
        let mut byte = [0u8];

        loop {
            match tcp_stream.read_exact(&mut byte) {
                Ok(()) => {
                    if byte[0] == b'\n' {
                        break;
                    }

                    buffer.push(byte[0]);
                }
                Err(error) => return Err(error),
            }
        }

        if let Ok(user_input) = String::from_utf8(buffer) {
            // If user types "STOP" service should stop accepting new connections
            if user_input == "STOP" {
                let _ = tcp_stream.write_all(b"Stopping the service ...\n");
                return Ok(LoopStep::Break);
            }

            // Reverse user input and send it back
            let reversed_user_input = user_input.chars().rev().collect::<String>();

            let _ = writeln!(tcp_stream, ">> {}", reversed_user_input);
            let _ = tcp_stream.flush();
        }

        Ok(LoopStep::Next)
    }
}

impl Service {
    fn new(socket: &str) -> Self {
        Self {
            tcp_listener: TcpListener::bind(socket).unwrap(),
        }
    }
}

fn main() -> io::Result<()> {
    // Create new cancellable service ($ nc 127.0.0.1 6556)
    let service = Service::new("127.0.0.1:6556");

    // Spawn execution loop in a new thread
    let handle = service.spawn();

    // Wait 1 minute and cancel the service
    let cancel_handle = handle.cancel_handle();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(60));
        cancel_handle.cancel();
    });

    // Continue with execution loop
    handle.wait()
}
