use std::net::TcpStream;
use std::io::{stdin, Read, Write};

// In main rather than secondary : tree, stats

// checkpoint doesn't work here. Bring it to `handle_stream`

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8081")?;
    loop {
        let mut input = String::new();
        print!("> ");
        stdin().read_line(&mut input)?;

        let trimmed = input.trim();

        stream.write_all(input.as_bytes())?;
        stream.flush()?; // make sure it's sent

        if trimmed == "quit" {
            break;
        }

        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            println!("Server closed connection");
            break;
        }

        let response = String::from_utf8_lossy(&buffer[..n]);
        println!("Server: {}", response);
    }
    Ok(())
}
