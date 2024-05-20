use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::io::BufReader;
use std::thread;
use std::fs;

const STATUS_200: &str = "HTTP/1.1 200 OK";
const STATUS_404: &str = "HTTP/1.1 404 Not Found";

fn get_request_url(request_string: &str) -> &str {
    let request_headers: Vec<&str> = request_string.split(' ').collect();
    request_headers[1]
}

fn echo(endpoint: &str) -> String {
    let headers = format!("Content-Type: text/plain\r\nContent-Length: {}\r\n", endpoint.len());
    let response = [STATUS_200, &headers, endpoint];
    response.join("\r\n")
}

fn get_user_agent(buffer: &mut BufReader<&mut TcpStream>) -> String {
    let mut buf_iter = buffer.lines();
    let user_agent = loop {
        if let Some(Ok(header_string)) = buf_iter.next() {
            if header_string == "\r\n" {
                // could not find user agent header
                break String::new();
            }
            let header_k_v = header_string.split_once(": ").unwrap();
            if header_k_v.0 == "User-Agent" {
                let user_agent = String::new() + header_k_v.1;
                break user_agent;
            }
        }
    };

    let headers = format!("Content-Type: text/plain\r\nContent-Length: {}\r\n", user_agent.len());
    let response = [STATUS_200, &headers, &user_agent];
    response.join("\r\n")
}

fn get_file(filename: &str, dir: &str) -> std::io::Result<String> {
    let filepath = String::new() + dir + "/" + filename;
    let file = fs::read(filepath)?;
    let headers = format!("Content-Type: application/octet-stream\r\nContent-Length: {}\r\n", file.len());
    let response = [STATUS_200, &headers, &String::from_utf8(file).unwrap()];
    Ok(response.join("\r\n"))
}

fn get_dir(mut args: std::env::Args) -> String {
    for arg in args.by_ref()  {
        if arg == "--directory"{
            break;
        }
    };

    if let Some(directory) = args.next() {
        return directory;
    }
    String::from("/")
}


fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    println!("Listening on port 4221");

    for stream in listener.incoming() {
        thread::spawn(move|| {
            match stream {
                Ok(mut stream) => {
                    let mut buffer = BufReader::new(&mut stream);
                    let mut req_line_string = String::new();
                    let _ = buffer.read_line(&mut req_line_string).unwrap();
                    let url = get_request_url(&req_line_string);
                    let url_parts: Vec<&str> = url.split_terminator('/').collect();
                    println!("{:?}", url_parts);
                    let response = match url_parts[..] {
                        [""] => STATUS_200.to_string() + "\r\n\r\n",
                        ["", "echo", endpoint] => echo(endpoint),
                        ["", "user-agent"] => get_user_agent(&mut buffer),
                        ["", "files", filename] => match get_file(filename, &get_dir(std::env::args())) {
                            Ok(response) => response,
                            _ => STATUS_404.to_string() + "\r\n\r\n",
                        },
                        _ => STATUS_404.to_string() + "\r\n\r\n",
                    };

                    let _ = stream.write(response.as_bytes());
                }
                Err(e) => {
                    println!("error: {}", e);
                }
            }
        });
    }
}
