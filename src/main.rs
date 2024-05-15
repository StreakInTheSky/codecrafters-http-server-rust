use std::io::prelude::*;
use std::net::TcpListener;
use std::io::BufReader;

fn get_request_url(request_string: &str) -> &str {
    let request_headers: Vec<&str> = request_string.split(' ').collect();
    request_headers[1]
}

fn main() {
     let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
     println!("Listening on port 4221");

     for stream in listener.incoming() {
         match stream {
             Ok(mut stream) => {
                 let mut buf = BufReader::new(&mut stream);
                 let mut string = String::new();
                 let _ = buf.read_line(&mut string).unwrap();
                 let url = get_request_url(&string);

                 let response = match url {
                     "/" => "HTTP/1.1 200 OK\r\n\r\n",
                     _ => "HTTP/1.1 404 Not Found\r\n\r\n",
                 };

                 let _ = stream.write(response.as_bytes());
             }
             Err(e) => {
                 println!("error: {}", e);
             }
         }
     }
}
