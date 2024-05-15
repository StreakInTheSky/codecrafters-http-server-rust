use std::io::Write;
use std::net::TcpListener;

fn main() {
     let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
     for stream in listener.incoming() {
         match stream {
             Ok(mut stream) => {
                 let response = "HTTP/1.1 200 OK\r\n\r\n";
                 let _ = stream.write(response.as_bytes());
             }
             Err(e) => {
                 println!("error: {}", e);
             }
         }
     }
}
