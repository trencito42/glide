use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

pub fn start() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let port = listener.local_addr().map_err(|error| error.to_string())?.port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let _ = serve(stream);
            }
        }
    });
    Ok(port)
}

fn serve(mut stream: TcpStream) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
    let mut buffer = [0_u8; 4096];
    let count = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..count]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let bare = path.split('?').next().unwrap_or("/");
    let (status, body) = match document(bare) {
        Some(body) => ("200 OK", body),
        None => ("404 Not Found", "Not found"),
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    Ok(())
}

fn document(path: &str) -> Option<&'static str> {
    match path {
        "/persist.html" => Some(include_str!("../../../../fixtures/sites/persist.html")),
        "/login.html" => Some(include_str!("../../../../fixtures/sites/login.html")),
        "/ipc-probe.html" => Some(include_str!("../../../../fixtures/sites/ipc-probe.html")),
        _ => None,
    }
}
